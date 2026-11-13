import init, { InteractiveDocument } from "./pkg/math_core_wasm.js";

/**
 * A math snippet, in the form the converter takes it.
 * @typedef {{ latex: string, displayStyle: boolean }} Snippet
 */

/**
 * The options of the converter.
 * @typedef {ConstructorParameters<typeof InteractiveDocument>[0]} Options
 */

/**
 * A piece of a paragraph: either plain text or a math snippet.
 * @typedef {{ text: string } | Snippet} Part
 */

/**
 * A paragraph of the document.
 * @typedef {object} Paragraph
 * @property {string} source The text of the paragraph, as it was typed.
 * @property {Snippet[]} snippets The math snippets in `source`, in order.
 * @property {HTMLElement[]} slots The elements which show the snippets; one per snippet.
 * @property {HTMLElement} element The element of the whole paragraph.
 * @property {HTMLElement} view The element which shows the rendered paragraph.
 */

const INITIAL_PARAGRAPHS = [
  String.raw`Commands which are defined with \gdef can be used in all the math after the definition. This paragraph defines two of them, in a formula which shows nothing. $\gdef\RR{\mathbb{R}} \gdef\dd{\,\mathrm{d}}$`,
  String.raw`Let $f \colon \RR \to \RR$ be continuously differentiable. The fundamental theorem of calculus, which is equation $\eqref{eq:ftc}$ below, relates $f$ to its derivative $f'$.`,
  String.raw`$$\begin{equation} \int_a^b f'(x) \dd x = f(b) - f(a) \label{eq:ftc} \end{equation}$$`,
  String.raw`An equation without a number has no effect on the rest of the document: $$\sum_{k=1}^n k = \frac{n(n+1)}{2}$$`,
  String.raw`An environment like align numbers each of its rows: $$\begin{align} e^{ix} &= \cos x + i \sin x \label{eq:euler} \\ e^{i\pi} + 1 &= 0 \label{eq:identity} \end{align}$$`,
  String.raw`The identity $\eqref{eq:identity}$ follows from $\eqref{eq:euler}$ with $x = \pi \in \RR$, and $\eqref{eq:ftc}$ has nothing to do with it.`,
];

/** @type {Paragraph[]} */
const paragraphs = [];

/** @type {InteractiveDocument} */
let converter;

const documentElement = /** @type {HTMLElement} */ (
  document.getElementById("document")
);
const statusElement = /** @type {HTMLElement} */ (
  document.getElementById("status")
);
const optionsForm = /** @type {HTMLFormElement} */ (
  document.getElementById("optionsForm")
);
/** The button at the end of the document, which the paragraphs are inserted in front of. */
const addButton = document.createElement("button");

/**
 * Split the text of a paragraph into plain text and math snippets.
 *
 * `$$…$$` is block math and `$…$` is inline math. `\$` is a dollar sign, in text as well as
 * in math. A delimiter which is never closed is plain text.
 * @param {string} source
 * @returns {Part[]}
 */
function splitParagraph(source) {
  /** @type {Part[]} */
  const parts = [];
  let text = "";
  let position = 0;
  while (position < source.length) {
    if (source.startsWith("\\$", position)) {
      text += "$";
      position += 2;
      continue;
    }
    if (source[position] !== "$") {
      text += source[position];
      position += 1;
      continue;
    }
    const delimiter = source.startsWith("$$", position) ? "$$" : "$";
    const start = position + delimiter.length;
    const end = findClosingDelimiter(source, start, delimiter);
    if (end === -1) {
      text += source.slice(position);
      break;
    }
    if (text) {
      parts.push({ text });
      text = "";
    }
    parts.push({
      latex: source.slice(start, end),
      displayStyle: delimiter === "$$",
    });
    position = end + delimiter.length;
  }
  if (text) {
    parts.push({ text });
  }
  return parts;
}

/**
 * @param {string} source
 * @param {number} from
 * @param {string} delimiter
 * @returns {number} The position of the delimiter, or -1 if there is none.
 */
function findClosingDelimiter(source, from, delimiter) {
  for (let position = from; position < source.length; position++) {
    if (source[position] === "\\") {
      // Skip the character after a backslash, which may be a dollar sign.
      position++;
    } else if (source.startsWith(delimiter, position)) {
      return position;
    }
  }
  return -1;
}

/**
 * @param {Snippet} a
 * @param {Snippet} b
 */
function sameSnippet(a, b) {
  return a.latex === b.latex && a.displayStyle === b.displayStyle;
}

/**
 * The index which the first snippet of a paragraph has in the converter.
 * @param {Paragraph} paragraph
 */
function snippetOffset(paragraph) {
  let offset = 0;
  for (const other of paragraphs) {
    if (other === paragraph) {
      break;
    }
    offset += other.snippets.length;
  }
  return offset;
}

/**
 * Replace the snippets of a paragraph in the converter, and update the snippets of the whole
 * document which the converter reports as changed.
 * @param {Paragraph} paragraph
 * @param {Snippet[]} newSnippets
 * @returns {{ start: number, changed: Uint32Array, milliseconds: number }} The index of the
 *   first snippet of the paragraph, and what the converter reported.
 */
function replaceSnippets(paragraph, newSnippets) {
  const oldSnippets = paragraph.snippets;
  // Leave out the snippets at the start and at the end which are the same as before, so
  // that the converter only hears about the ones which were edited.
  const shorter = Math.min(oldSnippets.length, newSnippets.length);
  let same = 0;
  while (same < shorter && sameSnippet(oldSnippets[same], newSnippets[same])) {
    same++;
  }
  let sameAtEnd = 0;
  while (
    sameAtEnd < shorter - same &&
    sameSnippet(
      oldSnippets[oldSnippets.length - 1 - sameAtEnd],
      newSnippets[newSnippets.length - 1 - sameAtEnd],
    )
  ) {
    sameAtEnd++;
  }

  const start = snippetOffset(paragraph);
  const before = performance.now();
  const changed = converter.splice(
    start + same,
    oldSnippets.length - same - sameAtEnd,
    newSnippets.slice(same, newSnippets.length - sameAtEnd),
  );
  const milliseconds = performance.now() - before;
  paragraph.snippets = newSnippets;
  return { start, changed, milliseconds };
}

/**
 * Show the output of the given snippets, and highlight them for a moment.
 * @param {Uint32Array} changed Indices of snippets in the converter.
 */
function showChangedSnippets(changed) {
  const slots = paragraphs.flatMap((paragraph) => paragraph.slots);
  for (const index of changed) {
    const slot = slots[index];
    slot.innerHTML = converter.get(index);
    const flashColor = getComputedStyle(slot).getPropertyValue("--flash-color");
    slot.animate(
      [{ backgroundColor: flashColor }, { backgroundColor: "transparent" }],
      { duration: 1500, easing: "ease-in" },
    );
  }
}

/**
 * @param {Uint32Array} changed
 * @param {number} milliseconds
 */
function showStatus(changed, milliseconds) {
  const total = converter.length;
  const snippets = total === 1 ? "snippet" : "snippets";
  statusElement.classList.remove("error");
  statusElement.textContent =
    `Last edit: ${changed.length} of ${total} ${snippets} updated ` +
    `(${milliseconds.toFixed(2)} ms in the converter).`;
}

/**
 * Take over a new text for a paragraph: build its view, and bring the math of the document up
 * to date.
 * @param {Paragraph} paragraph
 * @param {string} source
 */
function commitParagraph(paragraph, source) {
  const parts = splitParagraph(source);
  const newSnippets = parts.filter((part) => "latex" in part);
  const { start, changed, milliseconds } = replaceSnippets(
    paragraph,
    newSnippets,
  );
  paragraph.source = source;

  // The view is built from scratch, so every snippet of this paragraph needs its output,
  // whether it has changed or not.
  paragraph.slots = [];
  paragraph.view.replaceChildren();
  for (const part of parts) {
    if ("text" in part) {
      paragraph.view.append(part.text);
      continue;
    }
    const slot = document.createElement("span");
    slot.className = part.displayStyle ? "math block" : "math inline";
    slot.innerHTML = converter.get(start + paragraph.slots.length);
    paragraph.slots.push(slot);
    paragraph.view.append(slot);
  }

  showChangedSnippets(changed);
  showStatus(changed, milliseconds);
}

/**
 * @param {Paragraph} paragraph
 */
function removeParagraph(paragraph) {
  const { changed, milliseconds } = replaceSnippets(paragraph, []);
  paragraphs.splice(paragraphs.indexOf(paragraph), 1);
  paragraph.element.remove();
  showChangedSnippets(changed);
  showStatus(changed, milliseconds);
}

/**
 * Swap the view of a paragraph for a text area with its source. The paragraph is rendered
 * again when the text area loses the focus.
 * @param {Paragraph} paragraph
 */
function editParagraph(paragraph) {
  const textarea = document.createElement("textarea");
  textarea.value = paragraph.source;
  textarea.spellcheck = false;
  textarea.rows = 1;
  const fitHeight = () => {
    textarea.style.height = "auto";
    textarea.style.height = `${textarea.scrollHeight + 2}px`;
  };
  textarea.addEventListener("input", fitHeight);
  textarea.addEventListener("keydown", (event) => {
    if (
      event.key === "Escape" ||
      (event.key === "Enter" && (event.metaKey || event.ctrlKey))
    ) {
      event.preventDefault();
      textarea.blur();
    }
  });
  textarea.addEventListener("blur", () => {
    const source = textarea.value;
    textarea.remove();
    if (source.trim() === "") {
      // A paragraph without any text is not kept.
      removeParagraph(paragraph);
      return;
    }
    paragraph.view.hidden = false;
    commitParagraph(paragraph, source);
  });

  paragraph.view.hidden = true;
  paragraph.view.after(textarea);
  fitHeight();
  // The lines break differently once the font of the text area has arrived.
  document.fonts.ready.then(fitHeight);
  textarea.focus();
}

/**
 * Create an empty paragraph in front of the paragraph at `index`, or at the end.
 * @param {number} index
 * @returns {Paragraph}
 */
function createParagraph(index) {
  const element = document.createElement("section");
  element.className = "paragraph";

  const insertButton = document.createElement("button");
  insertButton.type = "button";
  insertButton.className = "insert";
  insertButton.title = "Add a paragraph here";
  insertButton.setAttribute("aria-label", "Add a paragraph here");
  insertButton.innerHTML = "<span>+</span>";

  const view = document.createElement("div");
  view.className = "view";
  view.tabIndex = 0;

  const deleteButton = document.createElement("button");
  deleteButton.type = "button";
  deleteButton.className = "delete";
  deleteButton.title = "Remove this paragraph";
  deleteButton.setAttribute("aria-label", "Remove this paragraph");
  deleteButton.textContent = "×";

  element.append(insertButton, view, deleteButton);

  /** @type {Paragraph} */
  const paragraph = { source: "", snippets: [], slots: [], element, view };

  insertButton.addEventListener("click", () => {
    editParagraph(createParagraph(paragraphs.indexOf(paragraph)));
  });
  view.addEventListener("click", (event) => {
    // A click on a reference should follow the link, not open the editor.
    if (event.target instanceof Element && event.target.closest("a")) {
      return;
    }
    editParagraph(paragraph);
  });
  view.addEventListener("keydown", (event) => {
    if (event.key === "Enter" && event.target === view) {
      event.preventDefault();
      editParagraph(paragraph);
    }
  });
  // The paragraph which is being edited is removed by its text area when that loses the
  // focus with nothing in it, so the button must not take the focus away from it.
  deleteButton.addEventListener("mousedown", (event) => event.preventDefault());
  deleteButton.addEventListener("click", () => {
    const textarea = element.querySelector("textarea");
    if (textarea) {
      textarea.value = "";
      textarea.blur();
    } else {
      removeParagraph(paragraph);
    }
  });

  const next = paragraphs[index];
  documentElement.insertBefore(element, next ? next.element : addButton);
  paragraphs.splice(index, 0, paragraph);
  return paragraph;
}

/**
 * Create the converter with the options of the form, and convert the whole document with it.
 */
function createConverter() {
  const options = new FormData(optionsForm);
  /** @type {Options} */
  const config = {
    // `get` returns the error message of a snippet as HTML, instead of throwing.
    throwOnError: false,
    tags: /** @type {Options["tags"]} */ (options.get("tags")),
    globalGroup: options.has("globalGroup"),
  };
  if (converter) {
    converter.free();
  }
  converter = new InteractiveDocument(config);
  for (const paragraph of paragraphs) {
    const { source } = paragraph;
    paragraph.snippets = [];
    commitParagraph(paragraph, source);
  }
  // Everything has just been converted, so there is no point in highlighting it.
  for (const animation of documentElement.getAnimations({ subtree: true })) {
    animation.cancel();
  }
  statusElement.textContent = `${converter.length} snippets converted.`;
}

async function main() {
  await init();

  addButton.type = "button";
  addButton.className = "add";
  addButton.textContent = "+ Add a paragraph";
  addButton.addEventListener("click", () => {
    editParagraph(createParagraph(paragraphs.length));
  });
  documentElement.append(addButton);

  for (const source of INITIAL_PARAGRAPHS) {
    createParagraph(paragraphs.length).source = source;
  }
  createConverter();
  optionsForm.addEventListener("change", createConverter);
}

main().catch((error) => {
  statusElement.classList.add("error");
  statusElement.textContent = `The converter could not be loaded: ${error}`;
  console.error(error);
});
