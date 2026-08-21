use std::borrow::Cow;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use similar::TextDiff;

use math_core::{LatexToMathML, MathCoreConfig, MathDisplay, PrettyPrint};

/// Snippets from <https://en.wikipedia.org/wiki/Help:Displaying_a_formula>.
///
/// The numbers are the positions in the original list; the gaps are snippets we didn't port.
const CONVERTIBLE: &[(u16, &str)] = &[
    (1, r"\alpha"),
    (2, r"f(x) = x^2"),
    (3, r"\{1,e,\pi\}"),
    (4, r"|z + 1| \leq 2"),
    (5, r"\# \$ \% \wedge \& \_ \{ \} \sim \backslash"),
    (6, r"\dot{a}, \ddot{a}, \acute{a}, \grave{a}"),
    (7, r"\dot{a}, \ddot{a}, \acute{a}, \grave{a}"),
    (8, r"\check{a}, \breve{a}, \tilde{a}, \bar{a}"),
    (9, r"\hat{a}, \widehat{a}, \vec{a}"),
    (10, r"\exp_a b = a^b, \exp b = e^b, 10^m"),
    (11, r"\ln c, \lg d = \log e, \log_{10} f"),
    (12, r"\sin a, \cos b, \tan c, \cot d, \sec e, \csc f"),
    (13, r"\arcsin h, \arccos i, \arctan j"),
    (14, r"\sinh k, \cosh l, \tanh m, \coth n"),
    (
        15,
        r"\operatorname{sh}k, \operatorname{ch}l, \operatorname{th}m, \operatorname{coth}n",
    ),
    (16, r"\sgn r, \left\vert s \right\vert"),
    (17, r"\min(x,y), \max(x,y)"),
    (18, r"\min x, \max y, \inf s, \sup t"),
    (19, r"\lim u, \liminf v, \limsup w"),
    (20, r"\dim p, \deg q, \det m, \ker\phi"),
    (21, r"\Pr j, \hom l, \lVert z \rVert, \arg z"),
    (22, r"dt, \mathrm{d}t, \partial t, \nabla\psi"),
    (
        23,
        r"dy/dx, \mathrm{d}y/\mathrm{d}x, \frac{dy}{dx}, \frac{\mathrm{d}y}{\mathrm{d}x}, \frac{\partial^2} {\partial x_1\partial x_2}y",
    ),
    (
        24,
        r"\prime, \backprime, f^\prime, f', f'', f^{(3)}, \dot y, \ddot y",
    ),
    (
        25,
        r"\infty, \aleph, \complement,\backepsilon, \eth, \Finv, \hbar",
    ),
    (
        26,
        r"\Im, \imath, \jmath, \Bbbk, \ell, \mho, \wp, \Re, \circledS, \S, \P, \text\AA",
    ),
    (27, r"s_k \equiv 0 \pmod{m}"),
    (28, r"a \bmod b"),
    (29, r"\gcd(m, n), \operatorname{lcm}(m, n)"),
    (
        31,
        r"\surd, \sqrt{2}, \sqrt[n]{2}, \sqrt[3]{\frac{x^3+y^3}{2}}",
    ),
    (32, r"+, -, \pm, \mp, \dotplus"),
    (33, r"\times, \div, \divideontimes, /, \backslash"),
    (34, r"\cdot, * \ast, \star, \circ, \bullet"),
    (35, r"\boxplus, \boxminus, \boxtimes, \boxdot"),
    (36, r"\oplus, \ominus, \otimes, \oslash, \odot"),
    (37, r"\circleddash, \circledcirc, \circledast"),
    (38, r"\bigoplus, \bigotimes, \bigodot"),
    (39, r"\{ \}, \text\O \empty \emptyset, \varnothing"),
    (40, r"\in, \notin \not\in, \ni, \not\ni"),
    (41, r"\cap, \Cap, \sqcap, \bigcap"),
    (
        42,
        r"\cup, \Cup, \sqcup, \bigcup, \bigsqcup, \uplus, \biguplus",
    ),
    (44, r"\subset, \Subset, \sqsubset"),
    (45, r"\supset, \Supset, \sqsupset"),
    (
        46,
        r"\subseteq, \nsubseteq, \subsetneq, \varsubsetneq, \sqsubseteq",
    ),
    (
        47,
        r"\supseteq, \nsupseteq, \supsetneq, \varsupsetneq, \sqsupseteq",
    ),
    (48, r"\subseteqq, \nsubseteqq, \subsetneqq, \varsubsetneqq"),
    (49, r"\supseteqq, \nsupseteqq, \supsetneqq, \varsupsetneqq"),
    (50, r"=, \ne, \neq, \equiv, \not\equiv"),
    (
        51,
        r"\doteq, \doteqdot, \overset{\underset{\mathrm{def}}{}}{=}, :=",
    ),
    (54, r"<, \nless, \ll, \not\ll, \lll, \not\lll, \lessdot"),
    (
        55,
        r"\le, \leq, \lneq, \leqq, \nleq, \nleqq, \lneqq, \lvertneqq",
    ),
    (
        56,
        r"\ge, \geq, \gneq, \geqq, \ngeq, \ngeqq, \gneqq, \gvertneqq",
    ),
    (
        57,
        r"\lessgtr, \lesseqgtr, \lesseqqgtr, \gtrless, \gtreqless, \gtreqqless",
    ),
    (58, r"\leqslant, \nleqslant, \eqslantless"),
    (59, r"\geqslant, \ngeqslant, \eqslantgtr"),
    (60, r"\lesssim, \lnsim, \lessapprox, \lnapprox"),
    (61, r"\gtrsim, \gnsim, \gtrapprox, \gnapprox"),
    (62, r"\prec, \nprec, \preceq, \npreceq, \precneqq"),
    (63, r"\succ, \nsucc, \succeq, \nsucceq, \succneqq"),
    (64, r"\preccurlyeq, \curlyeqprec"),
    (65, r"\succcurlyeq, \curlyeqsucc"),
    (66, r"\precsim, \precnsim, \precapprox, \precnapprox"),
    (67, r"\succsim, \succnsim, \succapprox, \succnapprox"),
    (
        69,
        r"\perp, \angle, \sphericalangle, \measuredangle, 45^\circ",
    ),
    (
        70,
        r"\Box, \square, \blacksquare, \diamond, \Diamond, \lozenge, \blacklozenge,\bigstar",
    ),
    (71, r"\bigcirc, \triangle, \bigtriangleup, \bigtriangledown"),
    (72, r"\vartriangle, \triangledown"),
    (
        73,
        r"\blacktriangle, \blacktriangledown, \blacktriangleleft, \blacktriangleright",
    ),
    (74, r"\forall, \exists, \nexists"),
    (75, r"\therefore, \because, \And"),
    (76, r"\lor \vee, \curlyvee, \bigvee"),
    (77, r"\land \wedge, \curlywedge, \bigwedge"),
    (
        78,
        r"\bar{q}, \bar{abc}, \overline{q}, \overline{abc}, \\ \lnot \neg, \not\operatorname{R},\bot,\top",
    ),
    (79, r"\vdash \dashv, \vDash, \Vdash, \models"),
    (80, r"\Vvdash \nvdash \nVdash \nvDash \nVDash"),
    (81, r"\ulcorner \urcorner \llcorner \lrcorner"),
    (82, r"\Rrightarrow, \Lleftarrow"),
    (83, r"\Rightarrow, \nRightarrow, \Longrightarrow, \implies"),
    (84, r"\Leftarrow, \nLeftarrow, \Longleftarrow"),
    (
        85,
        r"\Leftrightarrow, \nLeftrightarrow, \Longleftrightarrow, \iff",
    ),
    (86, r"\Uparrow, \Downarrow, \Updownarrow"),
    (87, r"\rightarrow \to, \nrightarrow, \longrightarrow"),
    (88, r"\leftarrow \gets, \nleftarrow, \longleftarrow"),
    (
        89,
        r"\leftrightarrow, \nleftrightarrow, \longleftrightarrow",
    ),
    (90, r"\uparrow, \downarrow, \updownarrow"),
    (91, r"\nearrow, \swarrow, \nwarrow, \searrow"),
    (92, r"\mapsto, \longmapsto"),
    (
        93,
        r"\rightharpoonup \rightharpoondown \leftharpoonup \leftharpoondown \upharpoonleft \upharpoonright \downharpoonleft \downharpoonright \rightleftharpoons \leftrightharpoons",
    ),
    (
        94,
        r"\curvearrowleft \circlearrowleft \Lsh \upuparrows \rightrightarrows \rightleftarrows \rightarrowtail \looparrowright",
    ),
    (
        95,
        r"\curvearrowright \circlearrowright \Rsh \downdownarrows \leftleftarrows \leftrightarrows \leftarrowtail \looparrowleft",
    ),
    (
        96,
        r"\hookrightarrow \hookleftarrow \multimap \leftrightsquigarrow \rightsquigarrow \twoheadrightarrow \twoheadleftarrow",
    ),
    (97, r"\amalg \P \S \% \dagger\ddagger\ldots\cdots"),
    (98, r"\smile \frown \wr \triangleleft \triangleright"),
    (
        99,
        r"\diamondsuit, \heartsuit, \clubsuit, \spadesuit, \Game, \flat, \natural, \sharp",
    ),
    (
        101,
        r"\eqcirc \circeq \triangleq \bumpeq\Bumpeq \doteqdot \risingdotseq \fallingdotseq",
    ),
    (
        102,
        r"\intercal \barwedge \veebar \doublebarwedge \between \pitchfork",
    ),
    (
        103,
        r"\vartriangleleft \ntriangleleft \vartriangleright \ntriangleright",
    ),
    (
        104,
        r"\trianglelefteq \ntrianglelefteq \trianglerighteq \ntrianglerighteq",
    ),
    (105, r"a^2, a^{x+3}"),
    (106, r"a_2"),
    (107, r"10^{30} a^{2+2} \\ a_{i,j} b_{f'}"),
    (108, r"x_2^3 \\ {x_2}^3"),
    (109, r"10^{10^{8}}"),
    (
        111,
        r"\overset{\alpha}{\omega} \\ \underset{\alpha}{\omega} \\ \overset{\alpha}{\underset{\gamma}{\omega}}\\ \stackrel{\alpha}{\omega}",
    ),
    (112, r"x', y'', f', f'' \\ x^\prime, y^{\prime\prime}"),
    (113, r"\dot{x}, \ddot{x}"),
    (
        114,
        r"\hat a \ \bar b \ \vec c \\ \overrightarrow{a b} \ \overleftarrow{c d}\\ \widehat{d e f} \\ \overline{g h i} \ \underline{j k l}",
    ),
    (115, r"\overset{\frown} {AB}"),
    (116, r"A \xleftarrow{n+\mu-1} B \xrightarrow[T]{n\pm i-1} C"),
    (117, r"\overbrace{ 1+2+\cdots+100 }^{5050}"),
    (118, r"\underbrace{ a+b+\cdots+z }_{26}"),
    (140, r"\frac{2}{4}=0.5\text{ or }{2 \over 4}=0.5"),
    (141, r"\frac{2}{4}=0.5"),
    (
        142,
        r"\dfrac{2}{4} = 0.5 \qquad \dfrac{2}{c + \dfrac{2}{d + \dfrac{2}{4}}} = a",
    ),
    (
        144,
        r"\cfrac{x}{1 + \cfrac{\cancel{y}} {\cancel{y}}} = \cfrac{x}{2}",
    ),
    (145, r"\binom{n}{k}"),
    (147, r"\dbinom{n}{k}"),
    (148, r"\begin{matrix} x & y \\ z & v \end{matrix}"),
    (149, r"\begin{vmatrix} x & y \\ z & v \end{vmatrix}"),
    (150, r"\begin{Vmatrix} x & y \\ z & v \end{Vmatrix}"),
    (
        151,
        r"\begin{bmatrix} 0 & \cdots & 0 \\ \vdots & \ddots & \vdots \\ 0 & \cdots & 0 \end{bmatrix}",
    ),
    (152, r"\begin{Bmatrix} x & y \\ z & v \end{Bmatrix}"),
    (153, r"\begin{pmatrix} x & y \\ z & v \end{pmatrix}"),
    (
        155,
        r"f(n) = \begin{cases} n/2, & \text{if }n\text{ is even} \\ 3n+1, & \text{if }n\text{ is odd} \end{cases}",
    ),
    (
        156,
        r"\begin{cases} 3x + 5y + z \\ 7x - 2y + 4z \\ -6x + 3y + 2z \end{cases}",
    ),
    (164, r"f(x) \,\!"),
    (
        165,
        r"\begin{array}{|c|c|c|} a & b & S \\ \hline 0 & 0 & 1 \\ 0 & 1 & 1 \\ 1 & 0 & 1 \\ 1 & 1 & 0 \\ \end{array}",
    ),
    (166, r"( \frac{1}{2} )^n"),
    (167, r"\left ( \frac{1}{2} \right )^n"),
    (168, r"\left ( \frac{a}{b} \right )"),
    (
        169,
        r"\left [ \frac{a}{b} \right ] \quad \left \lbrack \frac{a}{b} \right \rbrack",
    ),
    (
        170,
        r"\left \{ \frac{a}{b} \right \} \quad \left \lbrace \frac{a}{b} \right \rbrace",
    ),
    (171, r"\left \langle \frac{a}{b} \right \rangle"),
    (
        172,
        r"\left | \frac{a}{b} \right \vert \quad \left \Vert \frac{c}{d} \right \|",
    ),
    (
        173,
        r"\left \lfloor \frac{a}{b} \right \rfloor \quad \left \lceil \frac{c}{d} \right \rceil",
    ),
    (174, r"\left / \frac{a}{b} \right \backslash"),
    (
        175,
        r"\left\uparrow\frac{a}{b}\right\downarrow\; \left\Uparrow\frac{a}{b}\right\Downarrow\; \left \updownarrow \frac{a}{b} \right \Updownarrow",
    ),
    (176, r"\left [ 0,1 \right ) \left \langle \psi \right |"),
    (177, r"\left . \frac{A}{B} \right \} \to X"),
    (
        178,
        r"( \bigl( \Bigl( \biggl( \Biggl( \dots \Biggr] \biggr] \Bigr] \bigr] ]",
    ),
    (
        179,
        r"\{ \bigl\{ \Bigl\{ \biggl\{ \Biggl\{ \dots \Biggr\rangle \biggr\rangle \Bigr\rangle \bigr\rangle \rangle",
    ),
    (
        180,
        r"\| \big\| \Big\| \bigg\| \Bigg\| \dots \Bigg| \bigg| \Big| \big| |",
    ),
    (
        181,
        r"\lfloor \bigl\lfloor \Bigl\lfloor \biggl\lfloor \Biggl\lfloor \dots \Biggr\rceil \biggr\rceil \Bigr\rceil \bigr\rceil \rceil",
    ),
    (
        182,
        r"\uparrow \big\uparrow \Big\uparrow \bigg\uparrow \Bigg\uparrow \dots \Bigg\Downarrow \bigg\Downarrow \Big\Downarrow \big\Downarrow \Downarrow",
    ),
    (
        183,
        r"\updownarrow\big\updownarrow\Big\updownarrow \bigg\updownarrow \Bigg\updownarrow \dots \Bigg\Updownarrow \bigg\Updownarrow \Big \Updownarrow \big\Updownarrow \Updownarrow",
    ),
    (
        184,
        r"/ \big/ \Big/ \bigg/ \Bigg/ \dots \Bigg\backslash \bigg\backslash \Big \backslash \big\backslash \backslash",
    ),
    (
        185,
        r"\Alpha \Beta \Gamma \Delta \Epsilon \Zeta \Eta \Theta",
    ),
    (186, r"\Iota \Kappa \Lambda \Mu \Nu \Xi \Omicron \Pi"),
    (187, r"\Rho \Sigma \Tau \Upsilon \Phi \Chi \Psi \Omega"),
    (
        188,
        r"\alpha \beta \gamma \delta \epsilon \zeta \eta \theta",
    ),
    (189, r"\iota \kappa \lambda \mu \nu \xi \omicron \pi"),
    (190, r"\rho \sigma \tau \upsilon \phi \chi \psi \omega"),
    (
        191,
        r"\varGamma \varDelta \varTheta \varLambda \varXi \varPi \varSigma \varPhi \varUpsilon \varOmega",
    ),
    (
        192,
        r"\varepsilon \digamma \varkappa \varpi \varrho \varsigma \vartheta \varphi",
    ),
    (193, r"\aleph \beth \gimel \daleth"),
    (
        194,
        r"\mathbb{ABCDEFGHI} \\ \mathbb{JKLMNOPQR} \\ \mathbb{STUVWXYZ}",
    ),
    (
        195,
        r"\mathbf{ABCDEFGHI} \\ \mathbf{JKLMNOPQR} \\ \mathbf{STUVWXYZ} \\ \mathbf{abcdefghijklm} \\ \mathbf{nopqrstuvwxyz} \\ \mathbf{0123456789}",
    ),
    (
        196,
        r"\boldsymbol{\Alpha \Beta \Gamma \Delta \Epsilon \Zeta \Eta \Theta}",
    ),
    (
        197,
        r"\boldsymbol{\Iota \Kappa \Lambda \Mu \Nu \Xi \Omicron \Pi}",
    ),
    (
        198,
        r"\boldsymbol{\Rho \Sigma \Tau \Upsilon \Phi \Chi \Psi \Omega}",
    ),
    (
        199,
        r"\boldsymbol{\alpha \beta \gamma \delta \epsilon \zeta \eta \theta}",
    ),
    (
        200,
        r"\boldsymbol{\iota \kappa \lambda \mu \nu \xi \omicron \pi}",
    ),
    (
        201,
        r"\boldsymbol{\rho \sigma \tau \upsilon \phi \chi \psi \omega}",
    ),
    (202, r"\boldsymbol{\varepsilon\digamma\varkappa \varpi}"),
    (203, r"\boldsymbol{\varrho\varsigma\vartheta\varphi}"),
    (204, r"\mathit{0123456789}"),
    (
        205,
        r"\mathit{\Alpha \Beta \Gamma \Delta \Epsilon \Zeta \Eta \Theta}",
    ),
    (
        206,
        r"\mathit{\Iota \Kappa \Lambda \Mu \Nu \Xi \Omicron \Pi}",
    ),
    (
        207,
        r"\mathit{\Rho \Sigma \Tau \Upsilon \Phi \Chi \Psi \Omega}",
    ),
    (
        208,
        r"\boldsymbol{\varGamma \varDelta \varTheta \varLambda}",
    ),
    (
        209,
        r"\boldsymbol{\varXi \varPi \varSigma \varUpsilon \varOmega}",
    ),
    (
        210,
        r"\mathrm{ABCDEFGHI} \\ \mathrm{JKLMNOPQR} \\ \mathrm{STUVWXYZ} \\ \mathrm{abcdefghijklm} \\ \mathrm{nopqrstuvwxyz} \\ \mathrm{0123456789}",
    ),
    (
        211,
        r"\mathsf{ABCDEFGHI} \\ \mathsf{JKLMNOPQR} \\ \mathsf{STUVWXYZ} \\ \mathsf{abcdefghijklm} \\ \mathsf{nopqrstuvwxyz} \\ \mathsf{0123456789}",
    ),
    (
        212,
        r"\mathsf{\Alpha \Beta \Gamma \Delta \Epsilon \Zeta \Eta \Theta}",
    ),
    (
        213,
        r"\mathsf{\Iota \Kappa \Lambda \Mu \Nu \Xi \Omicron \Pi}",
    ),
    (
        214,
        r"\mathsf{\Rho \Sigma \Tau \Upsilon \Phi \Chi \Psi \Omega}",
    ),
    (
        215,
        r"\mathcal{ABCDEFGHI} \\ \mathcal{JKLMNOPQR} \\ \mathcal{STUVWXYZ} \\ \mathcal{abcdefghi} \\ \mathcal{jklmnopqr} \\ \mathcal{stuvwxyz}",
    ),
    (
        216,
        r"\mathfrak{ABCDEFGHI} \\ \mathfrak{JKLMNOPQR} \\ \mathfrak{STUVWXYZ} \\ \mathfrak{abcdefghi} \\ \mathfrak{jklmnopqr} \\ \mathfrak{stuvwxyz}",
    ),
    (217, r"{\scriptstyle\text{abcdefghijklm}}"),
    (218, r"x y z"),
    (219, r"\text{x y z}"),
    (220, r"\text{if} n \text{is even}"),
    (221, r"\text{if }n\text{ is even}"),
    (222, r"\text{if}~n\ \text{is even}"),
    (
        223,
        r"{\color{Blue}x^2}+{\color{Orange}2x}- {\color{LimeGreen}1}",
    ),
    (
        224,
        r"x_{1,2}=\frac{{\color{Blue}-b}\pm \sqrt{\color{Red}b^2-4ac}}{\color{Green}2a }",
    ),
    (
        225,
        r"{\color{Blue}x^2}+{\color{Orange}2x}- {\color{LimeGreen}1}",
    ),
    (
        226,
        r"\color{Blue}x^2\color{Black}+\color{Orange} 2x\color{Black}-\color{LimeGreen}1",
    ),
    (
        227,
        r"\color{Blue}{x^2}+\color{Orange}{2x}- \color{LimeGreen}{1}",
    ),
    (
        229,
        r"a \qquad b \\ a \quad b \\ a\ b \\ a \text{ } b \\ a\;b \\ a\,b \\ ab \\ a b \\ \mathit{ab} \\ a\!b",
    ),
    (230, r"| \uparrow \rangle"),
    (231, r"\left| \uparrow \right\rangle"),
    (232, r"| {\uparrow} \rangle"),
    (233, r"| \mathord\uparrow \rangle"),
    (234, r"\wideparen{AB}"),
    (235, r"\dddot{x}"),
    (237, r"\text{\sout{q}}"),
    (239, r"\text{\textsf{textual description}}"),
    (240, r"α π"),
    (241, r"ax^2 + bx + c = 0"),
    (242, r"x=\frac{-b\pm\sqrt{b^2-4ac}}{2a}"),
    (243, r"\left( \frac{\left(3-x\right) \times 2}{3-x} \right)"),
    (247, r"\det(\mathsf{A}-\lambda\mathsf{I}) = 0"),
    (250, r"u'' + p(x)u' + q(x)u=f(x),\quad x>a"),
    (
        251,
        r"|\bar{z}| = |z|, |(\bar{z})^n| = |z|^n, \arg(z^n) = n \arg(z)",
    ),
    (252, r"\lim_{z\to z_0} f(z)=f(z_0)"),
    (
        253,
        r"\phi_n(\kappa) = 0.033C_n^2\kappa^{-11/3}, \quad\frac{1}{L_0}\ll\kappa\ll\frac{1}{l_0}",
    ),
    (
        255,
        r"f(x) = \begin{cases} 1 & -1 \le x < 0 \\ \frac{1}{2} & x = 0 \\ 1 - x^2 & \text{otherwise} \end{cases}",
    ),
    (
        256,
        r"{}_pF_q(a_1,\dots,a_p;c_1,\dots,c_q;z) = \sum_{n=0}^\infty \frac{(a_1)_n\cdots(a_p)_n} {(c_1)_n\cdots(c_q)_n}\frac{z^n}{n!}",
    ),
    (259, r"S=dD\sin\alpha"),
    (
        260,
        r"V = \frac{1}{6} \pi h \left [ 3 \left ( r_1^2 + r_2^2 \right ) + h^2 \right ]",
    ),
    (
        261,
        r"\begin{align} u & = \tfrac{1}{\sqrt{2}}(x+y) \qquad & x &= \tfrac{1}{\sqrt{2}}(u+v) \\[0.6ex] v & = \tfrac{1}{\sqrt{2}}(x-y) \qquad & y &= \tfrac{1}{\sqrt{2}}(u-v) \end{align}",
    ),
];

/// Snippets that we cannot convert (yet). We run these as well, so that we notice when one of
/// them starts working after all.
const NOT_CONVERTIBLE: &[(u16, &str)] = &[
    (30, r"\mid, \nmid, \shortmid, \nshortmid"),
    (43, r"\setminus, \smallsetminus, \times"),
    (
        52,
        r"\sim, \nsim, \backsim, \thicksim, \simeq, \backsimeq, \eqsim, \cong, \ncong",
    ),
    (
        53,
        r"\approx, \thickapprox, \approxeq, \asymp, \propto, \varpropto",
    ),
    (
        68,
        r"\parallel, \nparallel, \shortparallel, \nshortparallel",
    ),
    (
        100,
        r"\diagup \diagdown \centerdot \ltimes \rtimes \leftthreetimes \rightthreetimes",
    ),
    (
        154,
        r"\bigl( \begin{smallmatrix} a&b\\ c&d \end{smallmatrix} \bigr)",
    ),
    (
        228,
        r"\definecolor{myorange}{rgb}{1,0.65,0.4} \color{myorange}e^{i \pi}\color{Black} + 1= 0",
    ),
    (238, r"\mathrlap{\,/}{=}"),
];

/// The section headings of the wiki page, each with the number of the first snippet below it.
/// Taken from <https://temml.org/tests/wiki-tests>, which lays out the same snippets.
///
/// A heading is emitted in front of the first snippet whose number is at least as large as the one
/// given here, so a section whose snippets we are all missing simply doesn't show up.
const HEADINGS: &[(u16, &str)] = &[
    (6, "Accents"),
    (10, "Functions"),
    (18, "Bounds"),
    (21, "Projections"),
    (22, "Differentials and derivatives"),
    (25, "Letter-like symbols or constants"),
    (27, "Modular arithmetic"),
    (31, "Radicals"),
    (32, "Operators"),
    (39, "Sets"),
    (50, "Relations"),
    (68, "Geometric"),
    (74, "Logic"),
    (82, "Arrows"),
    (97, "Special"),
    (100, "Unsorted"),
    (105, "Larger expressions"),
    (140, "Fractions, matrices, multiline"),
    (166, "Delimiters"),
    (185, "Greek Alphabet"),
    (193, "Hebrew symbols"),
    (194, "Blackboard bold"),
    (195, "Boldface"),
    (196, "Boldface Greek"),
    (204, "Italics"),
    (205, "Greek Italics"),
    (208, "Greek uppercase boldface italics"),
    (210, "Roman typeface"),
    (211, "Sans serif"),
    (212, "Sans serif Greek"),
    (215, "Calligraphy"),
    (216, "Fraktur"),
    (217, "Scriptstyle text"),
    (218, "Mixed text faces"),
    (223, "Color"),
    (229, "Spacing"),
    (234, "Wiki workarounds"),
    (241, "Examples of implemented TeX formulas"),
];

/// The accepted page, relative to the crate root.
const ACCEPTED_PAGE: &str = "../../playground/wiki_test.html";
/// Where we write the page when it differs from the accepted one.
const GENERATED_PAGE: &str = "tests/snapshots/wiki_test.html.new";

/// Everything before the table rows. The fonts and the `math` rule are the same as in
/// `examples/equations.rs`.
const HEADER: &str = r#"<!DOCTYPE html><html lang="en">
    <meta charset="UTF-8">
    <title>Wikipedia formula snippets</title>
    <link rel="stylesheet" href="./mathmlfixes.css" />
    <style>
        @font-face {
            font-family: "NewComputerModernMath Book";
            src: url('./fonts/NewCMMath-Book-prime-roundhand-vec-subset.woff2') format('woff2');
            font-display: swap;
        }
        @font-face {
            font-family: "NewComputerModern Book";
            src: url("./fonts/NewCM10-Book.woff2") format("woff2");
            font-display: swap;
        }
        @font-face {
            font-family: "NewComputerModern Mono";
            src: url("./fonts/NewCMMono10-Book.woff2") format("woff2");
            font-display: swap;
        }
        math {
            font-family: "NewComputerModernMath Book", math;
            mtext {
                font-family: "NewComputerModern Book", serif;
                code {
                    font-family: "NewComputerModern Mono", monospace;
                }
            }
        }
        table {
            border-collapse: collapse;
        }
        td {
            border: 1px solid #ccc;
            padding: 0.4em 0.7em;
            vertical-align: middle;
        }
        td:first-child {
            text-align: right;
            font-family: monospace;
        }
        th {
            border: 1px solid #ccc;
            padding: 0.7em;
            text-align: left;
            font-size: 1.15em;
            background-color: #f0f0f0;
        }
        nav ul {
            columns: 3 18em;
            list-style: none;
            margin: 0 0 2em;
            padding: 0;
        }
        nav li {
            line-height: 1.8;
            break-inside: avoid;
        }
        code {
            white-space: pre-wrap;
        }
    </style>
<body>
"#;

/// Everything after the table rows.
const FOOTER: &str = r#"    </table>
</body></html>
"#;

/// Convert all snippets, render the convertible ones into an HTML page and compare that page
/// against the accepted one in `playground/`.
///
/// If the two differ, the new page is written to `tests/snapshots/wiki_test.html.new` and the
/// difference is printed. To accept it, run:
///
/// ```sh
/// mv crates/math-core/tests/snapshots/wiki_test.html.new playground/wiki_test.html
/// ```
#[test]
fn wiki_test() {
    // Both the headings and the rows are emitted in one pass over `CONVERTIBLE`.
    assert!(
        CONVERTIBLE.is_sorted_by_key(|&(num, _)| num),
        "`CONVERTIBLE` must be sorted by number"
    );
    assert!(
        HEADINGS.is_sorted_by_key(|&(num, _)| num),
        "`HEADINGS` must be sorted by number"
    );

    let converter = LatexToMathML::new(MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        macros: vec![
            ("sgn".to_owned(), "\\operatorname{sgn}".to_owned()),
            ("wideparen".to_owned(), "\\overparen".to_owned()),
        ],
        ..Default::default()
    })
    .unwrap();

    // The table of contents can only list the sections that actually have rows, so we don't know
    // it until the loop below has run; that's why the rows are collected separately.
    let mut toc = String::new();
    let mut rows = String::from("    <table>\n");
    let mut headings = HEADINGS.iter().peekable();
    for &(num, latex) in CONVERTIBLE {
        // Emit the heading that this snippet falls under. If several are pending, only the last
        // one gets a row: the earlier sections have no snippets that we can convert.
        let mut section = None;
        while let Some(&&(start, title)) = headings.peek() {
            if start > num {
                break;
            }
            headings.next();
            section = Some(title);
        }
        if let Some(title) = section {
            let id = slug(title);
            writeln!(
                rows,
                "        <tr id=\"{id}\"><th colspan=\"3\">{}</th></tr>",
                escape(title)
            )
            .unwrap();
            writeln!(
                toc,
                "            <li><a href=\"#{id}\">{}</a></li>",
                escape(title)
            )
            .unwrap();
        }

        let latex = gather_line_breaks(latex);
        let mathml = converter
            .convert_with_local_state(&latex, MathDisplay::Block)
            .unwrap_or_else(|e| panic!("snippet {num} failed to convert: `{latex}`\n{e}"))
            .mathml;
        writeln!(rows, "        <tr id=\"n{num}\">").unwrap();
        writeln!(rows, "            <td><a href=\"#n{num}\">{num}</a></td>").unwrap();
        writeln!(rows, "            <td><code>{}</code></td>", escape(&latex)).unwrap();
        rows.push_str("            <td>\n");
        push_indented(&mut rows, &mathml, "                ");
        rows.push_str("            </td>\n        </tr>\n");
    }

    let mut generated = String::from(HEADER);
    generated.push_str("    <nav>\n        <ul>\n");
    generated.push_str(&toc);
    generated.push_str("        </ul>\n    </nav>\n");
    generated.push_str(&rows);
    generated.push_str(FOOTER);

    for &(num, latex) in NOT_CONVERTIBLE {
        assert!(
            converter
                .convert_with_local_state(&gather_line_breaks(latex), MathDisplay::Block)
                .is_err(),
            "snippet {num} converts now; move it to `CONVERTIBLE`: `{latex}`"
        );
    }

    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let generated_path = crate_root.join(GENERATED_PAGE);
    // A missing accepted page is treated as an empty one, so the whole page shows up as new.
    let accepted = fs::read_to_string(crate_root.join(ACCEPTED_PAGE)).unwrap_or_default();

    if accepted == generated {
        // Get rid of a stale page that an earlier, failing run may have left behind.
        let _ = fs::remove_file(&generated_path);
        return;
    }
    fs::write(&generated_path, &generated).expect("failed to write the new page");
    let diff = TextDiff::from_lines(accepted.as_str(), generated.as_str());
    print!(
        "{}",
        diff.unified_diff()
            .header("playground/wiki_test.html", GENERATED_PAGE)
    );
    panic!(
        "the wiki test page changed; to accept the new one, run:\n    \
         mv crates/math-core/tests/snapshots/wiki_test.html.new playground/wiki_test.html"
    );
}

/// Put `latex` into a `gather*` environment if it has line breaks that would otherwise be lost.
///
/// Like TeX, we ignore `\\` outside of an environment, so these snippets would all end up on a
/// single line. Snippets that bring their own environment are left alone: their `\\` already does
/// something, and `align` for instance cannot be nested inside `gather*`.
fn gather_line_breaks(latex: &str) -> Cow<'_, str> {
    if latex.contains(r"\\") && !latex.contains(r"\begin{") {
        Cow::Owned(format!(r"\begin{{gather*}} {latex} \end{{gather*}}"))
    } else {
        Cow::Borrowed(latex)
    }
}

/// Turn a section title into an id we can link to, e.g. "Greek Alphabet" -> "greek-alphabet".
fn slug(title: &str) -> String {
    let mut out = String::with_capacity(title.len());
    for c in title.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
        } else if !out.is_empty() && !out.ends_with('-') {
            out.push('-');
        }
    }
    out
}

/// Escape the characters that have a special meaning in HTML.
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

/// Append `text` to `out`, putting `indent` in front of every line.
fn push_indented(out: &mut String, text: &str, indent: &str) {
    for line in text.lines() {
        out.push_str(indent);
        out.push_str(line);
        out.push('\n');
    }
}
