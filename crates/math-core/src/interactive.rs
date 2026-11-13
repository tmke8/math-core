//! Converting the math of a document which is being edited.

use alloc::boxed::Box;
use alloc::rc::Rc;
use alloc::string::String;
use alloc::vec::Vec;
use core::mem;
use core::ops::Range;

use kstring::KString;
use mathml_renderer::{arena::Arena, ast::Node};

use crate::{
    ConvertResult, EmitterConfig, FxHashMap, LatexError, LatexToMathML, MacroParseError,
    MathCoreConfig, MathDisplay, ParseResult, ParserConfig, custom_cmds::CustomCmds, emit,
    global_state::GlobalState, parse,
};

/// Interface for rendering the math in a document which changes over time.
///
/// The challenge is that snippets of a document depend on each other: an equation number depends on
/// how many numbered equations come before it, an `\eqref` depends on the equation it refers to,
/// and a command which was defined with `\gdef` (or with `\newcommand`, when
/// [`global_group`](MathCoreConfig::global_group) is set) is available in every snippet after its
/// definition. Editing one snippet can therefore change what *other* snippets look like.
///
/// An `InteractiveDocument` holds all the snippets of a document, in the order in which they appear
/// in the document, together with their MathML. When the snippets are edited, with
/// [`insert`](Self::insert), [`replace`](Self::replace), [`remove`](Self::remove) or
/// [`splice`](Self::splice), this struct works out which snippets are affected, converts those
/// again, and returns the indices of the snippets whose output is now a different one, so that the
/// application only has to update these. The output itself is available from [`get`](Self::get)
/// which expects an index as its argument.
///
/// After every edit, the output of each snippet should be the one which
/// [`LatexToMathML::convert_all`] would give for the snippets the document consists of at that
/// point. In particular, a reference to an equation in a later snippet is resolved, and a snippet
/// which fails to convert has the same effect on the snippets after it as it has in `convert_all`.
///
/// # Example
///
/// ```rust
/// use math_core::{InteractiveDocument, MathCoreConfig, MathDisplay};
///
/// let mut doc = InteractiveDocument::new(MathCoreConfig::default(), vec![]).unwrap();
/// let equation = |body: &str| format!(r"\begin{{equation}}{body}\end{{equation}}");
///
/// // A document with a reference, a numbered equation and an inline formula.
/// doc.splice(
///     0..0,
///     [
///         (r"\eqref{eq:b}".to_string(), MathDisplay::Inline),
///         (equation(r"b = 2 \label{eq:b}"), MathDisplay::Block),
///         ("x + y".to_string(), MathDisplay::Inline),
///     ],
/// );
/// assert!(doc.get(0).unwrap().as_ref().unwrap().mathml.contains("(1)"));
///
/// // A numbered equation is added at the start of the document. That makes the old equation
/// // the second one, so it has to be updated, and with it the reference to it. The inline
/// // formula at the end is untouched.
/// let changed = doc.insert(0, equation("a = 1"), MathDisplay::Block);
/// assert_eq!(changed, [0, 1, 2]);
/// assert!(doc.get(1).unwrap().as_ref().unwrap().mathml.contains("(2)"));
/// ```
#[derive(Debug)]
pub struct InteractiveDocument {
    emitter_cfg: EmitterConfig,
    parser_cfg: ParserConfig,
    /// The snippets of the document, in "document order".
    snippets: Vec<SnippetSource>,
    /// What we know about each of the `snippets`. This vector has the same length as `snippets`.
    ///
    /// The two vectors are separate because the nodes of a snippet borrow from its source, so the
    /// sources have to stay untouched while the snippets are being converted, whereas the entries
    /// of this vector are updated along the way.
    rendered: Vec<RenderResult>,
    /// The labels of all snippets taken together. If a label is defined more than once, the
    /// last definition wins.
    label_map: FxHashMap<KString, KString>,
    /// The state at the very start of the document.
    start: GlobalStateCheckpoint,
    scratch: Scratch,
}

#[derive(Debug)]
struct SnippetSource {
    latex: String,
    display: MathDisplay,
}

/// The global state between two snippets, as far as the parser reads it.
///
/// The parser also writes to the label map, but it never reads it; only the emitter does.
#[derive(Debug, Clone)]
struct GlobalStateCheckpoint {
    equation_count: usize,
    /// The commands which the snippets up to this point have defined for the ones after them.
    ///
    /// Few snippets define such commands, so this is shared: consecutive checkpoints point at
    /// the same store until a snippet changes it. Two checkpoints with the same pointer hold
    /// the same definitions, which is what makes it cheap to notice that an edit has not
    /// affected them.
    custom_cmds: Rc<CustomCmds>,
}

impl GlobalStateCheckpoint {
    fn same_custom_cmds(&self, other: &GlobalStateCheckpoint) -> bool {
        Rc::ptr_eq(&self.custom_cmds, &other.custom_cmds)
    }
}

/// The result of converting a snippet.
#[derive(Debug)]
struct RenderResult {
    /// The parts of the global state that the parser reads, as seen after this snippet.
    after: GlobalStateCheckpoint,
    /// The labels which this snippet defines, with the equation tags they resolve to.
    labels: FxHashMap<KString, KString>,
    /// Whether the output contains an `\eqref`, in which case the output depends on the labels of
    /// the whole document.
    has_references: bool,
    output: Result<ConvertResult, Box<LatexError>>,
}

/// The state the parser works on, which is kept around between conversions.
#[derive(Debug)]
struct Scratch {
    state: GlobalState,
    /// The store which `state.custom_cmds` is currently a copy of.
    ///
    /// The parser needs a store it can write to, but the stores of the checkpoints are
    /// shared, so the parser gets a copy. Remembering what it is a copy of means that the copy only
    /// has to be made again when we move to a part of the document with other definitions.
    base: Rc<CustomCmds>,
}

impl Scratch {
    /// Parse a snippet which starts in the state `before`.
    ///
    /// Returns the nodes of the snippet, the state after it and the labels it defines. If
    /// the snippet leaves behind the same definitions as `previous`, then the state after it
    /// shares that store instead of getting a new one.
    fn parse<'arena>(
        &mut self,
        snippet: &'arena SnippetSource,
        arena: &'arena Arena,
        parser_cfg: &'arena ParserConfig,
        before: &GlobalStateCheckpoint,
        previous: Option<&Rc<CustomCmds>>,
    ) -> (
        ParseResult<Vec<&'arena Node<'arena>>>,
        GlobalStateCheckpoint,
        FxHashMap<KString, KString>,
    ) {
        self.state.equation_count = before.equation_count;
        if !Rc::ptr_eq(&self.base, &before.custom_cmds) {
            self.state.custom_cmds.clone_from(&before.custom_cmds);
            self.base = Rc::clone(&before.custom_cmds);
        }
        let nodes = parse(
            &snippet.latex,
            arena,
            parser_cfg,
            &mut self.state,
            snippet.display,
        );
        // A snippet which fails keeps the effects it had up to that point, as it does in
        // `LatexToMathML::convert_all`.
        if self.state.custom_cmds.take_changed() {
            self.base = match previous {
                Some(previous) if **previous == self.state.custom_cmds => Rc::clone(previous),
                _ => Rc::new(self.state.custom_cmds.clone()),
            };
        }
        let after = GlobalStateCheckpoint {
            equation_count: self.state.equation_count,
            custom_cmds: Rc::clone(&self.base),
        };
        (nodes, after, mem::take(&mut self.state.label_map))
    }
}

impl InteractiveDocument {
    /// Create an empty document.
    ///
    /// The arguments are the same as for [`LatexToMathML::new`], and so is the error.
    pub fn new(
        config: MathCoreConfig,
        macros: Vec<(String, String)>,
    ) -> Result<Self, MacroParseError> {
        let LatexToMathML {
            emitter_cfg,
            parser_cfg,
            state,
        } = LatexToMathML::new(config, macros)?;
        let custom_cmds = Rc::new(CustomCmds::default());
        Ok(Self {
            emitter_cfg,
            parser_cfg,
            snippets: Vec::new(),
            rendered: Vec::new(),
            label_map: FxHashMap::default(),
            start: GlobalStateCheckpoint {
                equation_count: 0,
                custom_cmds: Rc::clone(&custom_cmds),
            },
            scratch: Scratch {
                state,
                base: custom_cmds,
            },
        })
    }

    /// The number of snippets in the document.
    pub fn len(&self) -> usize {
        self.snippets.len()
    }

    /// Whether the document has no snippets.
    pub fn is_empty(&self) -> bool {
        self.snippets.is_empty()
    }

    /// The current output of the snippet at `index`, or `None` if there is no such snippet.
    pub fn get(&self, index: usize) -> Option<&Result<ConvertResult, Box<LatexError>>> {
        self.rendered.get(index).map(|rendered| &rendered.output)
    }

    /// The source of the snippet at `index`, or `None` if there is no such snippet.
    pub fn source(&self, index: usize) -> Option<(&str, MathDisplay)> {
        self.snippets
            .get(index)
            .map(|snippet| (snippet.latex.as_str(), snippet.display))
    }

    /// Add a snippet in front of the one which is currently at `index`; with `index == len`,
    /// the snippet is added at the end.
    ///
    /// Returns the indices of the snippets whose output has changed; see
    /// [`splice`](Self::splice).
    ///
    /// # Panics
    ///
    /// Panics if `index > len`.
    pub fn insert(&mut self, index: usize, latex: String, display: MathDisplay) -> Vec<usize> {
        self.splice(index..index, [(latex, display)])
    }

    /// Replace the snippet at `index` with another one.
    ///
    /// Returns the indices of the snippets whose output has changed; see
    /// [`splice`](Self::splice).
    ///
    /// # Panics
    ///
    /// Panics if `index >= len`.
    pub fn replace(&mut self, index: usize, latex: String, display: MathDisplay) -> Vec<usize> {
        assert!(index < self.len(), "snippet index out of bounds");
        self.splice(index..index + 1, [(latex, display)])
    }

    /// Remove the snippet at `index`.
    ///
    /// Returns the indices of the snippets whose output has changed; see
    /// [`splice`](Self::splice).
    ///
    /// # Panics
    ///
    /// Panics if `index >= len`.
    pub fn remove(&mut self, index: usize) -> Vec<usize> {
        assert!(index < self.len(), "snippet index out of bounds");
        self.splice(index..index + 1, [])
    }

    /// Remove all snippets.
    pub fn clear(&mut self) {
        self.snippets.clear();
        self.rendered.clear();
        self.label_map.clear();
    }

    /// Replace the snippets in `range` with `snippets`, which may be more or fewer than the
    /// ones they replace.
    ///
    /// This is the general form of [`insert`](Self::insert), [`replace`](Self::replace) and
    /// [`remove`](Self::remove). When several snippets change at once, a single call is
    /// better than a sequence of calls, because the snippets which depend on the changed ones
    /// are then converted only once.
    ///
    /// Returns the indices of the snippets whose output has changed, in ascending order. The
    /// indices are the ones the snippets have *after* the call. They always include the new
    /// snippets; of the others, they include those whose output is a different one than
    /// before, which can be snippets on either side of `range`. A snippet which failed to
    /// convert both before and now is reported whenever it had to be looked at again, without
    /// checking whether the error is the same.
    ///
    /// # Panics
    ///
    /// Panics if the start of `range` is greater than its end or if its end is greater than
    /// `len`.
    pub fn splice<I>(&mut self, range: Range<usize>, snippets: I) -> Vec<usize>
    where
        I: IntoIterator<Item = (String, MathDisplay)>,
    {
        let Range { start, end } = range;
        assert!(start <= end, "start of snippet range is after its end");
        assert!(end <= self.len(), "snippet range out of bounds");

        let old_len = self.snippets.len();
        let new_snippets = snippets
            .into_iter()
            .map(|(latex, display)| SnippetSource { latex, display });
        drop(self.snippets.splice(start..end, new_snippets));
        // The new snippets are now at `start..fresh_end`.
        let fresh_end = start + self.snippets.len() - (old_len - (end - start));

        let placeholders = (start..fresh_end).map(|_| RenderResult {
            after: self.start.clone(),
            labels: FxHashMap::default(),
            has_references: false,
            output: Ok(ConvertResult {
                mathml: String::new(),
                warnings: Default::default(),
            }),
        });
        let removed: Vec<RenderResult> = self.rendered.splice(start..end, placeholders).collect();

        self.update(start, fresh_end, &removed)
    }

    /// Bring the document up to date after the snippets at `start..fresh_end` have replaced
    /// the ones in `removed`.
    fn update(&mut self, start: usize, fresh_end: usize, removed: &[RenderResult]) -> Vec<usize> {
        let Self {
            emitter_cfg,
            parser_cfg,
            snippets,
            rendered,
            label_map,
            start: doc_start,
            scratch,
        } = self;
        let snippets = snippets.as_slice();
        let parser_cfg = &*parser_cfg;

        // As in `LatexToMathML::convert_all`, everything is parsed before anything is
        // emitted, because the labels have to be known by then.
        let arena = Arena::new();
        let mut parsed: Vec<(usize, ParseResult<Vec<&Node<'_>>>)> = Vec::new();

        // The state in front of the snippet we are looking at: what it is now, and what it
        // was before the edit.
        let mut now = match start.checked_sub(1) {
            Some(prev) => rendered[prev].after.clone(),
            None => doc_start.clone(),
        };
        let mut old = match removed.last() {
            Some(last) => last.after.clone(),
            None => now.clone(),
        };
        let mut labels_changed = removed.iter().any(|removed| !removed.labels.is_empty());

        // First, the new snippets and then those after them, for as long as the edit has
        // changed the state they start from.
        for (index, snippet) in snippets.iter().enumerate().skip(start) {
            let slot = &mut rendered[index];
            let previous = if index < fresh_end {
                // A new snippet takes the place of a removed one. It often is that snippet,
                // with a small change, in which case it defines the same commands.
                removed.get(index - start)
            } else {
                let same_custom_cmds = now.same_custom_cmds(&old);
                if same_custom_cmds && now.equation_count == old.equation_count {
                    // The state is back to what it was, so nothing changes from here on.
                    break;
                }
                // A snippet which numbers no equation never looks at the equation count.
                let ignores_count = slot.after.equation_count == old.equation_count;
                old = slot.after.clone();
                if same_custom_cmds && ignores_count {
                    slot.after.equation_count = now.equation_count;
                    now = slot.after.clone();
                    continue;
                }
                Some(&*slot)
            };
            let previous = previous.map(|previous| &previous.after.custom_cmds);
            let (nodes, after, labels) = scratch.parse(snippet, &arena, parser_cfg, &now, previous);
            parsed.push((index, nodes));
            labels_changed |= labels != slot.labels;
            now = after.clone();
            slot.after = after;
            slot.labels = labels;
            // This also marks the snippet as one that has been parsed already, which the
            // loop over the references below relies on.
            slot.has_references = false;
        }

        if labels_changed {
            let mut new_label_map = FxHashMap::default();
            for rendered in rendered.iter() {
                let labels = rendered.labels.iter();
                new_label_map.extend(labels.map(|(label, tag)| (label.clone(), tag.clone())));
            }
            // A label which is defined twice is one example of a change which does not show.
            labels_changed = new_label_map != *label_map;
            *label_map = new_label_map;
        }
        if labels_changed {
            // The references can be anywhere in the document, including in front of the
            // edit. Such a snippet is parsed in the same state as last time, so its effect
            // on that state is the same as well; only what is emitted for it may differ.
            for (index, snippet) in snippets.iter().enumerate() {
                let slot = &rendered[index];
                if !slot.has_references {
                    continue;
                }
                let before = match index.checked_sub(1) {
                    Some(prev) => &rendered[prev].after,
                    None => &*doc_start,
                };
                let previous = Some(&slot.after.custom_cmds);
                let (nodes, _, _) = scratch.parse(snippet, &arena, parser_cfg, before, previous);
                parsed.push((index, nodes));
            }
        }

        let mut changed = Vec::new();
        for (index, nodes) in parsed {
            let snippet = &snippets[index];
            let slot = &mut rendered[index];
            let output = nodes.map(|nodes| {
                let (latex, display) = (snippet.latex.as_str(), snippet.display);
                let (result, has_references) =
                    emit(nodes, latex, display, label_map, &arena, emitter_cfg);
                slot.has_references = has_references;
                result
            });
            if output.is_err() {
                slot.has_references = false;
            }
            let is_new = (start..fresh_end).contains(&index);
            let is_changed = match (&slot.output, &output) {
                (Ok(old), Ok(new)) => old.mathml != new.mathml,
                _ => true,
            };
            slot.output = output;
            if is_new || is_changed {
                changed.push(index);
            }
        }
        // The snippets with references come after the others in `parsed`, whatever their
        // position in the document.
        changed.sort_unstable();
        changed
    }
}
