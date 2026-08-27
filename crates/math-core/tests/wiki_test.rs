use std::borrow::Cow;
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

use similar::TextDiff;

use math_core::{LatexToMathML, MathCoreConfig, MathDisplay, PrettyPrint};

/// Snippets from <https://en.wikipedia.org/wiki/Help:Displaying_a_formula>, numbered as on
/// <https://temml.org/tests/wiki-tests>, which lays out the same list.
const SNIPPETS: &[&str] = &[
    r"\alpha",
    r"f(x) = x^2",
    r"\{1,e,\pi\}",
    r"|z + 1| \leq 2",
    r"\# \$ \% \wedge \& \_ \{ \} \sim \backslash",
    r"\dot{a}, \ddot{a}, \acute{a}, \grave{a}",
    r"\dot{a}, \ddot{a}, \acute{a}, \grave{a}",
    r"\check{a}, \breve{a}, \tilde{a}, \bar{a}",
    r"\hat{a}, \widehat{a}, \vec{a}",
    r"\exp_a b = a^b, \exp b = e^b, 10^m",
    r"\ln c, \lg d = \log e, \log_{10} f",
    r"\sin a, \cos b, \tan c, \cot d, \sec e, \csc f",
    r"\arcsin h, \arccos i, \arctan j",
    r"\sinh k, \cosh l, \tanh m, \coth n",
    r"\operatorname{sh}k, \operatorname{ch}l, \operatorname{th}m, \operatorname{coth}n",
    r"\sgn r, \left\vert s \right\vert",
    r"\min(x,y), \max(x,y)",
    r"\min x, \max y, \inf s, \sup t",
    r"\lim u, \liminf v, \limsup w",
    r"\dim p, \deg q, \det m, \ker\phi",
    r"\Pr j, \hom l, \lVert z \rVert, \arg z",
    r"dt, \mathrm{d}t, \partial t, \nabla\psi",
    r"dy/dx, \mathrm{d}y/\mathrm{d}x, \frac{dy}{dx}, \frac{\mathrm{d}y}{\mathrm{d}x}, \frac{\partial^2} {\partial x_1\partial x_2}y",
    r"\prime, \backprime, f^\prime, f', f'', f^{(3)}, \dot y, \ddot y",
    r"\infty, \aleph, \complement,\backepsilon, \eth, \Finv, \hbar",
    r"\Im, \imath, \jmath, \Bbbk, \ell, \mho, \wp, \Re, \circledS, \S, \P, \text\AA",
    r"s_k \equiv 0 \pmod{m}",
    r"a \bmod b",
    r"\gcd(m, n), \operatorname{lcm}(m, n)",
    r"\mid, \nmid, \shortmid, \nshortmid",
    r"\surd, \sqrt{2}, \sqrt[n]{2}, \sqrt[3]{\frac{x^3+y^3}{2}}",
    r"+, -, \pm, \mp, \dotplus",
    r"\times, \div, \divideontimes, /, \backslash",
    r"\cdot, * \ast, \star, \circ, \bullet",
    r"\boxplus, \boxminus, \boxtimes, \boxdot",
    r"\oplus, \ominus, \otimes, \oslash, \odot",
    r"\circleddash, \circledcirc, \circledast",
    r"\bigoplus, \bigotimes, \bigodot",
    r"\{ \}, \text\O \empty \emptyset, \varnothing",
    r"\in, \notin \not\in, \ni, \not\ni",
    r"\cap, \Cap, \sqcap, \bigcap",
    r"\cup, \Cup, \sqcup, \bigcup, \bigsqcup, \uplus, \biguplus",
    r"\setminus, \smallsetminus, \times",
    r"\subset, \Subset, \sqsubset",
    r"\supset, \Supset, \sqsupset",
    r"\subseteq, \nsubseteq, \subsetneq, \varsubsetneq, \sqsubseteq",
    r"\supseteq, \nsupseteq, \supsetneq, \varsupsetneq, \sqsupseteq",
    r"\subseteqq, \nsubseteqq, \subsetneqq, \varsubsetneqq",
    r"\supseteqq, \nsupseteqq, \supsetneqq, \varsupsetneqq",
    r"=, \ne, \neq, \equiv, \not\equiv",
    r"\doteq, \doteqdot, \overset{\underset{\mathrm{def}}{}}{=}, :=",
    r"\sim, \nsim, \backsim, \thicksim, \simeq, \backsimeq, \eqsim, \cong, \ncong",
    r"\approx, \thickapprox, \approxeq, \asymp, \propto, \varpropto",
    r"<, \nless, \ll, \not\ll, \lll, \not\lll, \lessdot",
    r"\le, \leq, \lneq, \leqq, \nleq, \nleqq, \lneqq, \lvertneqq",
    r"\ge, \geq, \gneq, \geqq, \ngeq, \ngeqq, \gneqq, \gvertneqq",
    r"\lessgtr, \lesseqgtr, \lesseqqgtr, \gtrless, \gtreqless, \gtreqqless",
    r"\leqslant, \nleqslant, \eqslantless",
    r"\geqslant, \ngeqslant, \eqslantgtr",
    r"\lesssim, \lnsim, \lessapprox, \lnapprox",
    r"\gtrsim, \gnsim, \gtrapprox, \gnapprox",
    r"\prec, \nprec, \preceq, \npreceq, \precneqq",
    r"\succ, \nsucc, \succeq, \nsucceq, \succneqq",
    r"\preccurlyeq, \curlyeqprec",
    r"\succcurlyeq, \curlyeqsucc",
    r"\precsim, \precnsim, \precapprox, \precnapprox",
    r"\succsim, \succnsim, \succapprox, \succnapprox",
    r"\parallel, \nparallel, \shortparallel, \nshortparallel",
    r"\perp, \angle, \sphericalangle, \measuredangle, 45^\circ",
    r"\Box, \square, \blacksquare, \diamond, \Diamond, \lozenge, \blacklozenge,\bigstar",
    r"\bigcirc, \triangle, \bigtriangleup, \bigtriangledown",
    r"\vartriangle, \triangledown",
    r"\blacktriangle, \blacktriangledown, \blacktriangleleft, \blacktriangleright",
    r"\forall, \exists, \nexists",
    r"\therefore, \because, \And",
    r"\lor \vee, \curlyvee, \bigvee",
    r"\land \wedge, \curlywedge, \bigwedge",
    r"\bar{q}, \bar{abc}, \overline{q}, \overline{abc}, \\ \lnot \neg, \not\operatorname{R},\bot,\top",
    r"\vdash \dashv, \vDash, \Vdash, \models",
    r"\Vvdash \nvdash \nVdash \nvDash \nVDash",
    r"\ulcorner \urcorner \llcorner \lrcorner",
    r"\Rrightarrow, \Lleftarrow",
    r"\Rightarrow, \nRightarrow, \Longrightarrow, \implies",
    r"\Leftarrow, \nLeftarrow, \Longleftarrow",
    r"\Leftrightarrow, \nLeftrightarrow, \Longleftrightarrow, \iff",
    r"\Uparrow, \Downarrow, \Updownarrow",
    r"\rightarrow \to, \nrightarrow, \longrightarrow",
    r"\leftarrow \gets, \nleftarrow, \longleftarrow",
    r"\leftrightarrow, \nleftrightarrow, \longleftrightarrow",
    r"\uparrow, \downarrow, \updownarrow",
    r"\nearrow, \swarrow, \nwarrow, \searrow",
    r"\mapsto, \longmapsto",
    r"\rightharpoonup \rightharpoondown \leftharpoonup \leftharpoondown \upharpoonleft \upharpoonright \downharpoonleft \downharpoonright \rightleftharpoons \leftrightharpoons",
    r"\curvearrowleft \circlearrowleft \Lsh \upuparrows \rightrightarrows \rightleftarrows \rightarrowtail \looparrowright",
    r"\curvearrowright \circlearrowright \Rsh \downdownarrows \leftleftarrows \leftrightarrows \leftarrowtail \looparrowleft",
    r"\hookrightarrow \hookleftarrow \multimap \leftrightsquigarrow \rightsquigarrow \twoheadrightarrow \twoheadleftarrow",
    r"\amalg \P \S \% \dagger\ddagger\ldots\cdots",
    r"\smile \frown \wr \triangleleft \triangleright",
    r"\diamondsuit, \heartsuit, \clubsuit, \spadesuit, \Game, \flat, \natural, \sharp",
    r"\diagup \diagdown \centerdot \ltimes \rtimes \leftthreetimes \rightthreetimes",
    r"\eqcirc \circeq \triangleq \bumpeq\Bumpeq \doteqdot \risingdotseq \fallingdotseq",
    r"\intercal \barwedge \veebar \doublebarwedge \between \pitchfork",
    r"\vartriangleleft \ntriangleleft \vartriangleright \ntriangleright",
    r"\trianglelefteq \ntrianglelefteq \trianglerighteq \ntrianglerighteq",
    r"a^2, a^{x+3}",
    r"a_2",
    r"10^{30} a^{2+2} \\ a_{i,j} b_{f'}",
    r"x_2^3 \\ {x_2}^3",
    r"10^{10^{8}}",
    r"\sideset{_1^2}{_3^4}\prod_a^b \\ {}_1^2\!\Omega_3^4",
    r"\overset{\alpha}{\omega} \\ \underset{\alpha}{\omega} \\ \overset{\alpha}{\underset{\gamma}{\omega}}\\ \stackrel{\alpha}{\omega}",
    r"x', y'', f', f'' \\ x^\prime, y^{\prime\prime}",
    r"\dot{x}, \ddot{x}",
    r"\hat a \ \bar b \ \vec c \\ \overrightarrow{a b} \ \overleftarrow{c d}\\ \widehat{d e f} \\ \overline{g h i} \ \underline{j k l}",
    r"\overset{\frown} {AB}",
    r"A \xleftarrow{n+\mu-1} B \xrightarrow[T]{n\pm i-1} C",
    r"\overbrace{ 1+2+\cdots+100 }^{5050}",
    r"\underbrace{ a+b+\cdots+z }_{26}",
    r"\sum_{k=1}^N k^2",
    r"\textstyle \sum_{k=1}^N k^2",
    r"\frac{\sum_{k=1}^N k^2}{a}",
    r"\frac{\sum\limits^{^N}_{k=1} k^2}{a}",
    r"\prod_{i=1}^N x_i",
    r"\textstyle \prod_{i=1}^N x_i",
    r"\coprod_{i=1}^N x_i",
    r"\textstyle \coprod_{i=1}^N x_i",
    r"\lim_{n \to \infty}x_n",
    r"\textstyle \lim_{n \to \infty}x_n",
    r"\int\limits_{1}^{3}\frac{e^3/x}{x^2}\, dx",
    r"\int_{1}^{3}\frac{e^3/x}{x^2}\, dx",
    r"\textstyle \int\limits_{-N}^{N} e^x dx",
    r"\textstyle \int_{-N}^{N} e^x dx",
    r"\iint\limits_D dx\,dy",
    r"\iiint\limits_E dx\,dy\,dz",
    r"\iiiint\limits_F dx\,dy\,dz\,dt",
    r"\int_{(x,y)\in C} x^3\, dx + 4y^2\, dy",
    r"\oint_{(x,y)\in C} x^3\, dx + 4y^2\, dy",
    r"\bigcap_{i=1}^n E_i",
    r"\bigcup_{i=1}^n E_i",
    r"\frac{2}{4}=0.5\text{ or }{2 \over 4}=0.5",
    r"\frac{2}{4}=0.5",
    r"\dfrac{2}{4} = 0.5 \qquad \dfrac{2}{c + \dfrac{2}{d + \dfrac{2}{4}}} = a",
    r"\cfrac{2}{c+\cfrac{2}{d+\cfrac{2}{4}}} = a",
    r"\cfrac{x}{1 + \cfrac{\cancel{y}} {\cancel{y}}} = \cfrac{x}{2}",
    r"\binom{n}{k}",
    r"\tbinom{n}{k}",
    r"\dbinom{n}{k}",
    r"\begin{matrix} x & y \\ z & v \end{matrix}",
    r"\begin{vmatrix} x & y \\ z & v \end{vmatrix}",
    r"\begin{Vmatrix} x & y \\ z & v \end{Vmatrix}",
    r"\begin{bmatrix} 0 & \cdots & 0 \\ \vdots & \ddots & \vdots \\ 0 & \cdots & 0 \end{bmatrix}",
    r"\begin{Bmatrix} x & y \\ z & v \end{Bmatrix}",
    r"\begin{pmatrix} x & y \\ z & v \end{pmatrix}",
    r"\bigl( \begin{smallmatrix} a&b\\ c&d \end{smallmatrix} \bigr)",
    r"f(n) = \begin{cases} n/2, & \text{if }n\text{ is even} \\ 3n+1, & \text{if }n\text{ is odd} \end{cases}",
    r"\begin{cases} 3x + 5y + z \\ 7x - 2y + 4z \\ -6x + 3y + 2z \end{cases}",
    r"\begin{align} f(x) & = (a+b)^2 \\ & = a^2+2ab+b^2 \\ \end{align}",
    r"\begin{alignat}{2} f(x) & = (a+b)^2 \\ & = a^2+2ab+b^2 \\ \end{alignat}",
    r"\begin{alignat}{3} f(a,b) & = (a+b)^2 && = (a+b)(a+b) \\ & = a^2+ab+ba+b^2 && = a^2+2ab+b^2 \\ \end{alignat}",
    r"\begin{array}{lcl} z & = & a \\ f(x,y,z) & = & x + y + z \end{array}",
    r"\begin{array}{lcr} z & = & a \\ f(x,y,z) & = & x + y + z \end{array}",
    r"\begin{alignat}{4} F:\; && C(X) && \;\to\; & C(X) \\ && g && \;\mapsto\; & g^2 \end{alignat}",
    r"\begin{alignat}{4} F:\; && C(X) && \;\to\; && C(X) \\ && g && \;\mapsto\; && g^2 \end{alignat}",
    r"f(x) \,\!",
    r"\begin{array}{|c|c|c|} a & b & S \\ \hline 0 & 0 & 1 \\ 0 & 1 & 1 \\ 1 & 0 & 1 \\ 1 & 1 & 0 \\ \end{array}",
    r"( \frac{1}{2} )^n",
    r"\left ( \frac{1}{2} \right )^n",
    r"\left ( \frac{a}{b} \right )",
    r"\left [ \frac{a}{b} \right ] \quad \left \lbrack \frac{a}{b} \right \rbrack",
    r"\left \{ \frac{a}{b} \right \} \quad \left \lbrace \frac{a}{b} \right \rbrace",
    r"\left \langle \frac{a}{b} \right \rangle",
    r"\left | \frac{a}{b} \right \vert \quad \left \Vert \frac{c}{d} \right \|",
    r"\left \lfloor \frac{a}{b} \right \rfloor \quad \left \lceil \frac{c}{d} \right \rceil",
    r"\left / \frac{a}{b} \right \backslash",
    r"\left\uparrow\frac{a}{b}\right\downarrow\; \left\Uparrow\frac{a}{b}\right\Downarrow\; \left \updownarrow \frac{a}{b} \right \Updownarrow",
    r"\left [ 0,1 \right ) \left \langle \psi \right |",
    r"\left . \frac{A}{B} \right \} \to X",
    r"( \bigl( \Bigl( \biggl( \Biggl( \dots \Biggr] \biggr] \Bigr] \bigr] ]",
    r"\{ \bigl\{ \Bigl\{ \biggl\{ \Biggl\{ \dots \Biggr\rangle \biggr\rangle \Bigr\rangle \bigr\rangle \rangle",
    r"\| \big\| \Big\| \bigg\| \Bigg\| \dots \Bigg| \bigg| \Big| \big| |",
    r"\lfloor \bigl\lfloor \Bigl\lfloor \biggl\lfloor \Biggl\lfloor \dots \Biggr\rceil \biggr\rceil \Bigr\rceil \bigr\rceil \rceil",
    r"\uparrow \big\uparrow \Big\uparrow \bigg\uparrow \Bigg\uparrow \dots \Bigg\Downarrow \bigg\Downarrow \Big\Downarrow \big\Downarrow \Downarrow",
    r"\updownarrow\big\updownarrow\Big\updownarrow \bigg\updownarrow \Bigg\updownarrow \dots \Bigg\Updownarrow \bigg\Updownarrow \Big \Updownarrow \big\Updownarrow \Updownarrow",
    r"/ \big/ \Big/ \bigg/ \Bigg/ \dots \Bigg\backslash \bigg\backslash \Big \backslash \big\backslash \backslash",
    r"\Alpha \Beta \Gamma \Delta \Epsilon \Zeta \Eta \Theta",
    r"\Iota \Kappa \Lambda \Mu \Nu \Xi \Omicron \Pi",
    r"\Rho \Sigma \Tau \Upsilon \Phi \Chi \Psi \Omega",
    r"\alpha \beta \gamma \delta \epsilon \zeta \eta \theta",
    r"\iota \kappa \lambda \mu \nu \xi \omicron \pi",
    r"\rho \sigma \tau \upsilon \phi \chi \psi \omega",
    r"\varGamma \varDelta \varTheta \varLambda \varXi \varPi \varSigma \varPhi \varUpsilon \varOmega",
    r"\varepsilon \digamma \varkappa \varpi \varrho \varsigma \vartheta \varphi",
    r"\aleph \beth \gimel \daleth",
    r"\mathbb{ABCDEFGHI} \\ \mathbb{JKLMNOPQR} \\ \mathbb{STUVWXYZ}",
    r"\mathbf{ABCDEFGHI} \\ \mathbf{JKLMNOPQR} \\ \mathbf{STUVWXYZ} \\ \mathbf{abcdefghijklm} \\ \mathbf{nopqrstuvwxyz} \\ \mathbf{0123456789}",
    r"\boldsymbol{\Alpha \Beta \Gamma \Delta \Epsilon \Zeta \Eta \Theta}",
    r"\boldsymbol{\Iota \Kappa \Lambda \Mu \Nu \Xi \Omicron \Pi}",
    r"\boldsymbol{\Rho \Sigma \Tau \Upsilon \Phi \Chi \Psi \Omega}",
    r"\boldsymbol{\alpha \beta \gamma \delta \epsilon \zeta \eta \theta}",
    r"\boldsymbol{\iota \kappa \lambda \mu \nu \xi \omicron \pi}",
    r"\boldsymbol{\rho \sigma \tau \upsilon \phi \chi \psi \omega}",
    r"\boldsymbol{\varepsilon\digamma\varkappa \varpi}",
    r"\boldsymbol{\varrho\varsigma\vartheta\varphi}",
    r"\mathit{0123456789}",
    r"\mathit{\Alpha \Beta \Gamma \Delta \Epsilon \Zeta \Eta \Theta}",
    r"\mathit{\Iota \Kappa \Lambda \Mu \Nu \Xi \Omicron \Pi}",
    r"\mathit{\Rho \Sigma \Tau \Upsilon \Phi \Chi \Psi \Omega}",
    r"\boldsymbol{\varGamma \varDelta \varTheta \varLambda}",
    r"\boldsymbol{\varXi \varPi \varSigma \varUpsilon \varOmega}",
    r"\mathrm{ABCDEFGHI} \\ \mathrm{JKLMNOPQR} \\ \mathrm{STUVWXYZ} \\ \mathrm{abcdefghijklm} \\ \mathrm{nopqrstuvwxyz} \\ \mathrm{0123456789}",
    r"\mathsf{ABCDEFGHI} \\ \mathsf{JKLMNOPQR} \\ \mathsf{STUVWXYZ} \\ \mathsf{abcdefghijklm} \\ \mathsf{nopqrstuvwxyz} \\ \mathsf{0123456789}",
    r"\mathsf{\Alpha \Beta \Gamma \Delta \Epsilon \Zeta \Eta \Theta}",
    r"\mathsf{\Iota \Kappa \Lambda \Mu \Nu \Xi \Omicron \Pi}",
    r"\mathsf{\Rho \Sigma \Tau \Upsilon \Phi \Chi \Psi \Omega}",
    r"\mathcal{ABCDEFGHI} \\ \mathcal{JKLMNOPQR} \\ \mathcal{STUVWXYZ} \\ \mathcal{abcdefghi} \\ \mathcal{jklmnopqr} \\ \mathcal{stuvwxyz}",
    r"\mathfrak{ABCDEFGHI} \\ \mathfrak{JKLMNOPQR} \\ \mathfrak{STUVWXYZ} \\ \mathfrak{abcdefghi} \\ \mathfrak{jklmnopqr} \\ \mathfrak{stuvwxyz}",
    r"{\scriptstyle\text{abcdefghijklm}}",
    r"x y z",
    r"\text{x y z}",
    r"\text{if} n \text{is even}",
    r"\text{if }n\text{ is even}",
    r"\text{if}~n\ \text{is even}",
    r"{\color{Blue}x^2}+{\color{Orange}2x}- {\color{LimeGreen}1}",
    r"x_{1,2}=\frac{{\color{Blue}-b}\pm \sqrt{\color{Red}b^2-4ac}}{\color{Green}2a }",
    r"{\color{Blue}x^2}+{\color{Orange}2x}- {\color{LimeGreen}1}",
    r"\color{Blue}x^2\color{Black}+\color{Orange} 2x\color{Black}-\color{LimeGreen}1",
    r"\color{Blue}{x^2}+\color{Orange}{2x}- \color{LimeGreen}{1}",
    r"\definecolor{myorange}{rgb}{1,0.65,0.4} \color{myorange}e^{i \pi}\color{Black} + 1= 0",
    r"a \qquad b \\ a \quad b \\ a\ b \\ a \text{ } b \\ a\;b \\ a\,b \\ ab \\ a b \\ \mathit{ab} \\ a\!b",
    r"| \uparrow \rangle",
    r"\left| \uparrow \right\rangle",
    r"| {\uparrow} \rangle",
    r"| \mathord\uparrow \rangle",
    r"\wideparen{AB}",
    r"\dddot{x}",
    r"\operatorname*{median}_{j\,\ne\,i} X_{i,j}",
    r"\text{\sout{q}}",
    r"\mathrlap{\,/}{=}",
    r"\text{\textsf{textual description}}",
    r"α π",
    r"ax^2 + bx + c = 0",
    r"x=\frac{-b\pm\sqrt{b^2-4ac}}{2a}",
    r"\left( \frac{\left(3-x\right) \times 2}{3-x} \right)",
    r"S_{\text{new}} = S_{\text{old}} - \frac{ \left( 5-T \right) ^2} {2}",
    r"\int_a^x \int_a^s f(y)\,dy\,ds = \int_a^x f(y)(x-y)\,dy",
    r"\int_e^{\infty}\frac {1}{t(\ln t)^2}dt = \left. \frac{-1}{\ln t} \right\vert_e^\infty = 1",
    r"\det(\mathsf{A}-\lambda\mathsf{I}) = 0",
    r"\sum_{i=0}^{n-1} i",
    r"\sum_{m=1}^\infty\sum_{n=1}^\infty \frac{m^2 n}{3^m\left(m 3^n + n 3^m\right)}",
    r"u'' + p(x)u' + q(x)u=f(x),\quad x>a",
    r"|\bar{z}| = |z|, |(\bar{z})^n| = |z|^n, \arg(z^n) = n \arg(z)",
    r"\lim_{z\to z_0} f(z)=f(z_0)",
    r"\phi_n(\kappa) = 0.033C_n^2\kappa^{-11/3}, \quad\frac{1}{L_0}\ll\kappa\ll\frac{1}{l_0}",
    r"\phi_n(\kappa) = 0.033C_n^2\kappa^{-11/3}, \quad\frac{1}{L_0}\ll\kappa\ll\frac{1}{l_0}",
    r"f(x) = \begin{cases} 1 & -1 \le x < 0 \\ \frac{1}{2} & x = 0 \\ 1 - x^2 & \text{otherwise} \end{cases}",
    r"{}_pF_q(a_1,\dots,a_p;c_1,\dots,c_q;z) = \sum_{n=0}^\infty \frac{(a_1)_n\cdots(a_p)_n} {(c_1)_n\cdots(c_q)_n}\frac{z^n}{n!}",
    r"\frac{a}{b}\ \tfrac{a}{b}",
    r"S=dD\sin\alpha",
    r"V = \frac{1}{6} \pi h \left [ 3 \left ( r_1^2 + r_2^2 \right ) + h^2 \right ]",
    r"\begin{align} u & = \tfrac{1}{\sqrt{2}}(x+y) \qquad & x &= \tfrac{1}{\sqrt{2}}(u+v) \\[0.6ex] v & = \tfrac{1}{\sqrt{2}}(x-y) \qquad & y &= \tfrac{1}{\sqrt{2}}(u-v) \end{align}",
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
                span.math-core-sans-serif-font {
                    font-family: "NewComputerModern Sans", sans-serif;
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
        td.error {
            color: #c00;
            font-family: monospace;
        }
        math[display="block"] {
            /* Left align like the wiki page does. Firefox centers block math with `text-align`,
               Chrome with `justify-self`, so we have to override both. */
            text-align: left;
            justify-self: start;
        }
    </style>
<body>
"#;

/// Everything after the table rows.
const FOOTER: &str = r#"    </table>
</body></html>
"#;

/// Convert all snippets, render them into an HTML page and compare that page against the
/// accepted one in `playground/`.
///
/// If the two differ, the new page is written to `tests/snapshots/wiki_test.html.new` and the
/// difference is printed. To accept it, run:
///
/// ```sh
/// mv crates/math-core/tests/snapshots/wiki_test.html.new playground/wiki_test.html
/// ```
#[test]
fn wiki_test() {
    // Both the headings and the rows are emitted in one pass over `SNIPPETS`.
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
    for (num, latex) in SNIPPETS.iter().enumerate() {
        let num = num + 1; // 1-based numbering
        // Emit the heading that this snippet falls under. If several are pending, only the last
        // one gets a row: the earlier sections have no snippets in our list.
        let mut section = None;
        while let Some(&&(start, title)) = headings.peek() {
            if usize::from(start) > num {
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
        writeln!(rows, "        <tr id=\"n{num}\">").unwrap();
        writeln!(rows, "            <td><a href=\"#n{num}\">{num}</a></td>").unwrap();
        writeln!(rows, "            <td><code>{}</code></td>", escape(&latex)).unwrap();
        match converter.convert_with_local_state(&latex, MathDisplay::Block) {
            Ok(converted) => {
                rows.push_str("            <td>\n");
                push_indented(&mut rows, &converted.mathml, "                ");
                rows.push_str("            </td>\n");
            }
            Err(e) => {
                // Show the error where the output would go, so that the page records it too.
                writeln!(
                    rows,
                    "            <td class=\"error\">{}</td>",
                    escape(&e.to_string())
                )
                .unwrap();
            }
        }
        rows.push_str("        </tr>\n");
    }

    let mut generated = String::from(HEADER);
    generated.push_str("    <nav>\n        <ul>\n");
    generated.push_str(&toc);
    generated.push_str("        </ul>\n    </nav>\n");
    generated.push_str(&rows);
    generated.push_str(FOOTER);

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
