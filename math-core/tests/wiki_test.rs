use std::sync::LazyLock;

use insta::assert_snapshot;
use regex::Regex;
// use similar::{ChangeTag, TextDiff};

use math_core::{LatexToMathML, MathCoreConfig, MathDisplay, PrettyPrint};

#[test]
fn wiki_test() {
    let problems = [
        (r"\alpha", "<math><mi>α</mi></math>"),
        (
            r"f(x) = x^2",
            "<math><mrow><mi>f</mi><mo form=\"prefix\" stretchy=\"false\">(</mo><mi>x</mi><mo form=\"postfix\" stretchy=\"false\">)</mo><mo>=</mo></mrow><mrow><msup><mi>x</mi><mn>2</mn></msup></mrow></math>",
        ),
        (
            r"\{1,e,\pi\}",
            "<math><mrow><mo form=\"prefix\" stretchy=\"false\">{</mo><mn>1</mn><mo>,</mo><mi>e</mi><mo>,</mo><mi>π</mi><mo form=\"postfix\" stretchy=\"false\">}</mo></mrow></math>",
        ),
        (
            r"|z + 1| \leq 2",
            "<math><mrow><mi>|</mi><mi>z</mi><mo>+</mo></mrow><mrow><mn>1</mn><mi>|</mi><mo>≤</mo></mrow><mrow><mn>2</mn></mrow></math>",
        ),
        (
            r"\# \$ \% \wedge \& \_ \{ \} \sim \backslash",
            "<math><mrow><mi>#</mi><mi>$</mi><mi>%</mi><mo>∧</mo></mrow><mrow><mi>&amp;</mi><mi>_</mi><mo form=\"prefix\" stretchy=\"false\">{</mo><mo form=\"postfix\" stretchy=\"false\">}</mo><mo>∼</mo></mrow><mrow><mi>\\</mi></mrow></math>",
        ),
        (
            r"\dot{a}, \ddot{a}, \acute{a}, \grave{a}",
            "<math><mrow><mover><mi>a</mi><mo stretchy=\"false\" style=\"math-style:normal;math-depth:0;\">˙</mo></mover><mo>,</mo></mrow><mrow><mover><mi>a</mi><mo stretchy=\"false\" style=\"math-style:normal;math-depth:0;\">¨</mo></mover><mo>,</mo></mrow><mrow><mover><mi>a</mi><mo stretchy=\"false\" style=\"math-style:normal;math-depth:0;\">ˊ</mo></mover><mo>,</mo></mrow><mrow><mover><mi>a</mi><mo stretchy=\"false\" style=\"math-style:normal;math-depth:0;\">`</mo></mover></mrow></math>",
        ),
        (
            r"\dot{a}, \ddot{a}, \acute{a}, \grave{a}",
            "<math><mrow><mover><mi>a</mi><mo stretchy=\"false\" style=\"math-style:normal;math-depth:0;\">˙</mo></mover><mo>,</mo></mrow><mrow><mover><mi>a</mi><mo stretchy=\"false\" style=\"math-style:normal;math-depth:0;\">¨</mo></mover><mo>,</mo></mrow><mrow><mover><mi>a</mi><mo stretchy=\"false\" style=\"math-style:normal;math-depth:0;\">ˊ</mo></mover><mo>,</mo></mrow><mrow><mover><mi>a</mi><mo stretchy=\"false\" style=\"math-style:normal;math-depth:0;\">`</mo></mover></mrow></math>",
        ),
        (
            r"\check{a}, \breve{a}, \tilde{a}, \bar{a}",
            "<math><mrow><mover><mi>a</mi><mo stretchy=\"false\" style=\"math-style:normal;math-depth:0;\">ˇ</mo></mover><mo>,</mo></mrow><mrow><mover><mi>a</mi><mo stretchy=\"false\" style=\"math-style:normal;math-depth:0;\">˘</mo></mover><mo>,</mo></mrow><mrow><mover><mi>a</mi><mo stretchy=\"false\" style=\"math-style:normal;math-depth:0;\">~</mo></mover><mo>,</mo></mrow><mrow><mover><mi>a</mi><mo stretchy=\"false\" style=\"math-style:normal;math-depth:0;\">‾</mo></mover></mrow></math>",
        ),
        (
            r"\hat{a}, \widehat{a}, \vec{a}",
            "<math><mrow><mover><mi>a</mi><mo stretchy=\"false\" style=\"math-style:normal;math-depth:0;\">^</mo></mover><mo>,</mo></mrow><mrow><mover><mi>a</mi><mo stretchy=\"true\" style=\"math-style:normal;math-depth:0;\">^</mo></mover><mo>,</mo></mrow><mrow><mover><mi>a</mi><mo stretchy=\"false\" style=\"transform:scale(0.75) translate(10%, 30%);\">→</mo></mover></mrow></math>",
        ),
        (
            r"\exp_a b = a^b, \exp b = e^b, 10^m",
            "<math><mrow><msub><mi>exp</mi><mi>a</mi></msub><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mrow><mi>b</mi><mo>=</mo></mrow><mrow><msup><mi>a</mi><mi>b</mi></msup><mo>,</mo></mrow><mrow><mrow><mi>exp</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>b</mi><mo>=</mo></mrow><mrow><msup><mi>e</mi><mi>b</mi></msup><mo>,</mo></mrow><mrow><msup><mn>10</mn><mi>m</mi></msup></mrow></math>",
        ),
        (
            r"\ln c, \lg d = \log e, \log_{10} f",
            "<math><mrow><mrow><mi>ln</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>c</mi><mo>,</mo></mrow><mrow><mrow><mi>lg</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>d</mi><mo>=</mo></mrow><mrow><mrow><mi>log</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>e</mi><mo>,</mo></mrow><mrow><msub><mi>log</mi><mn>10</mn></msub><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mrow><mi>f</mi></mrow></math>",
        ),
        (
            r"\sin a, \cos b, \tan c, \cot d, \sec e, \csc f",
            "<math><mrow><mrow><mi>sin</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>a</mi><mo>,</mo></mrow><mrow><mrow><mi>cos</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>b</mi><mo>,</mo></mrow><mrow><mrow><mi>tan</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>c</mi><mo>,</mo></mrow><mrow><mrow><mi>cot</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>d</mi><mo>,</mo></mrow><mrow><mrow><mi>sec</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>e</mi><mo>,</mo></mrow><mrow><mrow><mi>csc</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>f</mi></mrow></math>",
        ),
        (
            r"\arcsin h, \arccos i, \arctan j",
            "<math><mrow><mrow><mi>arcsin</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>h</mi><mo>,</mo></mrow><mrow><mrow><mi>arccos</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>i</mi><mo>,</mo></mrow><mrow><mrow><mi>arctan</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>j</mi></mrow></math>",
        ),
        (
            r"\sinh k, \cosh l, \tanh m, \coth n",
            "<math><mrow><mrow><mi>sinh</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>k</mi><mo>,</mo></mrow><mrow><mrow><mi>cosh</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>l</mi><mo>,</mo></mrow><mrow><mrow><mi>tanh</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>m</mi><mo>,</mo></mrow><mrow><mrow><mi>coth</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>n</mi></mrow></math>",
        ),
        (
            r"\operatorname{sh}k, \operatorname{ch}l, \operatorname{th}m, \operatorname{coth}n",
            "<math><mrow><mi>sh</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mrow><mi>k</mi><mo>,</mo></mrow><mrow><mi>ch</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mrow><mi>l</mi><mo>,</mo></mrow><mrow><mi>th</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mrow><mi>m</mi><mo>,</mo></mrow><mrow><mi>coth</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mrow><mi>n</mi></mrow></math>",
        ),
        (
            r"\sgn r, \left\vert s \right\vert",
            "<math><mrow><mrow><mi>sgn</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>r</mi><mo>,</mo></mrow><mrow><mrow><mo fence=\"true\" form=\"prefix\">|</mo><mi>s</mi><mo fence=\"true\" form=\"postfix\">|</mo></mrow></mrow></math>",
        ),
        (
            r"\min(x,y), \max(x,y)",
            "<math><mrow><mrow><mi>min</mi><mo>⁡</mo></mrow><mo form=\"prefix\" stretchy=\"false\">(</mo><mi>x</mi><mo>,</mo><mi>y</mi><mo form=\"postfix\" stretchy=\"false\">)</mo><mo>,</mo></mrow><mrow><mrow><mi>max</mi><mo>⁡</mo></mrow><mo form=\"prefix\" stretchy=\"false\">(</mo><mi>x</mi><mo>,</mo><mi>y</mi><mo form=\"postfix\" stretchy=\"false\">)</mo></mrow></math>",
        ),
        (
            r"\min x, \max y, \inf s, \sup t",
            "<math><mrow><mrow><mi>min</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>x</mi><mo>,</mo></mrow><mrow><mrow><mi>max</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>y</mi><mo>,</mo></mrow><mrow><mrow><mi>inf</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>s</mi><mo>,</mo></mrow><mrow><mrow><mi>sup</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>t</mi></mrow></math>",
        ),
        (
            r"\lim u, \liminf v, \limsup w",
            "<math><mrow><mrow><mi>lim</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>u</mi><mo>,</mo></mrow><mrow><mi>lim inf</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mrow><mi>v</mi><mo>,</mo></mrow><mrow><mi>lim sup</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mrow><mi>w</mi></mrow></math>",
        ),
        (
            r"\dim p, \deg q, \det m, \ker\phi",
            "<math><mrow><mrow><mi>dim</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>p</mi><mo>,</mo></mrow><mrow><mrow><mi>deg</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>q</mi><mo>,</mo></mrow><mrow><mrow><mi>det</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>m</mi><mo>,</mo></mrow><mrow><mrow><mi>ker</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>ϕ</mi></mrow></math>",
        ),
        (
            r"\Pr j, \hom l, \lVert z \rVert, \arg z",
            "<math><mrow><mrow><mi>Pr</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>j</mi><mo>,</mo></mrow><mrow><mrow><mi>hom</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>l</mi><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">‖</mo></mrow><mrow><mi>z</mi><mo form=\"postfix\" stretchy=\"false\">‖</mo></mrow><mrow><mo>,</mo></mrow><mrow><mrow><mi>arg</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>z</mi></mrow></math>",
        ),
        (
            r"dt, \mathrm{d}t, \partial t, \nabla\psi",
            "<math><mrow><mi>d</mi><mi>t</mi><mo>,</mo></mrow><mrow><mrow><mi mathvariant=\"normal\">d</mi></mrow><mi>t</mi><mo>,</mo></mrow><mrow><mi>∂</mi><mi>t</mi><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">∇</mo><mi>ψ</mi></mrow></math>",
        ),
        (
            r"dy/dx, \mathrm{d}y/\mathrm{d}x, \frac{dy}{dx}, \frac{\mathrm{d}y}{\mathrm{d}x}, \frac{\partial^2} {\partial x_1\partial x_2}y",
            "<math><mrow><mi>d</mi><mi>y</mi><mi>/</mi><mi>d</mi><mi>x</mi><mo>,</mo></mrow><mrow><mrow><mi mathvariant=\"normal\">d</mi></mrow><mi>y</mi><mi>/</mi><mrow><mi mathvariant=\"normal\">d</mi></mrow><mi>x</mi><mo>,</mo></mrow><mrow><mfrac><mrow><mi>d</mi><mi>y</mi></mrow><mrow><mi>d</mi><mi>x</mi></mrow></mfrac><mo>,</mo></mrow><mrow><mfrac><mrow><mrow><mi mathvariant=\"normal\">d</mi></mrow><mi>y</mi></mrow><mrow><mrow><mi mathvariant=\"normal\">d</mi></mrow><mi>x</mi></mrow></mfrac><mo>,</mo></mrow><mrow><mfrac><msup><mi>∂</mi><mn>2</mn></msup><mrow><mi>∂</mi><msub><mi>x</mi><mn>1</mn></msub><mi>∂</mi><msub><mi>x</mi><mn>2</mn></msub></mrow></mfrac><mi>y</mi></mrow></math>",
        ),
        (
            r"\prime, \backprime, f^\prime, f', f'', f^{(3)}, \dot y, \ddot y",
            "<math><mrow><mo>′</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>‵</mo></mrow><mrow><mo>,</mo></mrow><mrow><msup><mi>f</mi><mo class=\"tml-prime prime-pad\">′</mo></msup><mo>,</mo></mrow><mrow><msup><mi>f</mi><mo class=\"tml-prime prime-pad\" lspace=\"0em\" rspace=\"0em\">′</mo></msup><mo>,</mo></mrow><mrow><msup><mi>f</mi><mrow><mo class=\"tml-prime prime-pad\" lspace=\"0em\" rspace=\"0em\">′</mo><mo lspace=\"0em\" rspace=\"0em\">′</mo></mrow></msup><mo>,</mo></mrow><mrow><msup><mi>f</mi><mrow><mo form=\"prefix\" lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">(</mo><mn>3</mn><mo form=\"postfix\" lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">)</mo></mrow></msup><mo>,</mo></mrow><mrow><mover><mi>y</mi><mo stretchy=\"false\" style=\"math-style:normal;math-depth:0;\">˙</mo></mover><mo>,</mo></mrow><mrow><mover><mi>y</mi><mo stretchy=\"false\" style=\"math-style:normal;math-depth:0;\">¨</mo></mover></mrow></math>",
        ),
        (
            r"\infty, \aleph, \complement,\backepsilon, \eth, \Finv, \hbar",
            "<math><mrow><mi>∞</mi><mo>,</mo></mrow><mrow><mi>ℵ</mi><mo>,</mo></mrow><mrow><mi>∁</mi><mo>,</mo></mrow><mrow><mo>∍</mo></mrow><mrow><mo>,</mo></mrow><mrow><mi>ð</mi><mo>,</mo></mrow><mrow><mi>Ⅎ</mi><mo>,</mo></mrow><mrow><mi>ℏ</mi></mrow></math>",
        ),
        (
            r"\Im, \imath, \jmath, \Bbbk, \ell, \mho, \wp, \Re, \circledS, \S, \P, \text\AA",
            "<math><mrow><mi>ℑ</mi><mo>,</mo></mrow><mrow><mi>ı</mi><mo>,</mo></mrow><mrow><mi>ȷ</mi><mo>,</mo></mrow><mrow><mi>𝕜</mi><mo>,</mo></mrow><mrow><mi>ℓ</mi><mo>,</mo></mrow><mrow><mi>℧</mi><mo>,</mo></mrow><mrow><mi>℘</mi><mo>,</mo></mrow><mrow><mi>ℜ</mi><mo>,</mo></mrow><mrow><mi>Ⓢ</mi><mo>,</mo></mrow><mrow><mi>§</mi><mo>,</mo></mrow><mrow><mi>¶</mi><mo>,</mo></mrow><mrow><mover><mi>A</mi><mo class=\"tml-capshift\" stretchy=\"false\" style=\"math-style:normal;math-depth:0;\">˚</mo></mover></mrow></math>",
        ),
        (
            r"s_k \equiv 0 \pmod{m}",
            "<math><mrow><msub><mi>s</mi><mi>k</mi></msub><mo>≡</mo></mrow><mrow><mn>0</mn><mo></mo><mspace width=\"0.4444em\"></mspace><mo form=\"prefix\" stretchy=\"false\">(</mo><mrow><mtext></mtext><mi>mod</mi></mrow><mspace width=\"0.3333em\"></mspace><mi>m</mi><mo form=\"postfix\" stretchy=\"false\">)</mo></mrow></math>",
        ),
        (
            r"a \bmod b",
            "<math><mrow><mi>a</mi><mo lspace=\"0.2222em\" rspace=\"0.2222em\">mod</mo></mrow><mrow><mi>b</mi></mrow></math>",
        ),
        (
            r"\gcd(m, n), \operatorname{lcm}(m, n)",
            "<math><mrow><mrow><mi>gcd</mi><mo>⁡</mo></mrow><mo form=\"prefix\" stretchy=\"false\">(</mo><mi>m</mi><mo>,</mo><mi>n</mi><mo form=\"postfix\" stretchy=\"false\">)</mo><mo>,</mo></mrow><mrow><mi>lcm</mi><mo>⁡</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">(</mo><mi>m</mi><mo>,</mo><mi>n</mi><mo form=\"postfix\" stretchy=\"false\">)</mo></mrow></math>",
        ),
        (
            r"\mid, \nmid, \shortmid, \nshortmid",
            "<math><mrow><mo lspace=\"0.22em\" rspace=\"0.22em\" stretchy=\"false\">|</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>∤</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo mathsize=\"70%\">∣</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo mathsize=\"70%\">∤</mo></mrow></math>",
        ),
        (
            r"\surd, \sqrt{2}, \sqrt[n]{2}, \sqrt[3]{\frac{x^3+y^3}{2}}",
            "<math><mrow><msqrt><mpadded width=\"0px\"><mphantom><mi>|</mi></mphantom></mpadded></msqrt><mo>,</mo></mrow><mrow><msqrt><mn>2</mn></msqrt><mo>,</mo></mrow><mrow><mroot><mn>2</mn><mi>n</mi></mroot><mo>,</mo></mrow><mrow><mroot><mfrac><mrow><msup><mi>x</mi><mn>3</mn></msup><mo>+</mo><msup><mi>y</mi><mn>3</mn></msup></mrow><mn>2</mn></mfrac><mn>3</mn></mroot></mrow></math>",
        ),
        (
            r"+, -, \pm, \mp, \dotplus",
            "<math><mrow><mo>+</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">−</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">±</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">∓</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">∔</mo></mrow></math>",
        ),
        (
            r"\times, \div, \divideontimes, /, \backslash",
            "<math><mrow><mo>×</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">÷</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">⋇</mo></mrow><mrow><mo>,</mo></mrow><mrow><mi>/</mi><mo>,</mo></mrow><mrow><mi>\\</mi></mrow></math>",
        ),
        (
            r"\cdot, * \ast, \star, \circ, \bullet",
            "<math><mrow><mo>⋅</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">*</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">∗</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">⋆</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">∘</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">∙</mo></mrow></math>",
        ),
        (
            r"\boxplus, \boxminus, \boxtimes, \boxdot",
            "<math><mrow><mo>⊞</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">⊟</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">⊠</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">⊡</mo></mrow></math>",
        ),
        (
            r"\oplus, \ominus, \otimes, \oslash, \odot",
            "<math><mrow><mo>⊕︎</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">⊖</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">⊗</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">⊘</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">⊙</mo></mrow></math>",
        ),
        (
            r"\circleddash, \circledcirc, \circledast",
            "<math><mrow><mo>⊝</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">⊚</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">⊛</mo></mrow></math>",
        ),
        (
            r"\bigoplus, \bigotimes, \bigodot",
            "<math><mrow><mo movablelimits=\"false\">⨁</mo><mo>,</mo></mrow><mrow><mo movablelimits=\"false\">⨂</mo><mo>,</mo></mrow><mrow><mo movablelimits=\"false\">⨀</mo></mrow></math>",
        ),
        (
            r"\{ \}, \text\O \empty \emptyset, \varnothing",
            "<math><mrow><mo form=\"prefix\" stretchy=\"false\">{</mo><mo form=\"postfix\" stretchy=\"false\">}</mo><mo>,</mo></mrow><mrow><mi>Ø</mi><mi>∅</mi><mi>∅</mi><mo>,</mo></mrow><mrow><mi>⌀</mi></mrow></math>",
        ),
        (
            r"\in, \notin \not\in, \ni, \not\ni",
            "<math><mrow><mo>∈</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>∉</mo></mrow><mrow><mo>∉</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>∋</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>∌</mo></mrow></math>",
        ),
        (
            r"\cap, \Cap, \sqcap, \bigcap",
            "<math><mrow><mo>∩</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">⋒</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">⊓</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo movablelimits=\"false\">⋂</mo></mrow></math>",
        ),
        (
            r"\cup, \Cup, \sqcup, \bigcup, \bigsqcup, \uplus, \biguplus",
            "<math><mrow><mo>∪</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">⋓</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">⊔</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo movablelimits=\"false\">⋃</mo><mo>,</mo></mrow><mrow><mo movablelimits=\"false\">⨆</mo><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">⊎</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo movablelimits=\"false\">⨄</mo></mrow></math>",
        ),
        (
            r"\setminus, \smallsetminus, \times",
            "<math><mrow><mo>∖</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">∖</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">×</mo></mrow></math>",
        ),
        (
            r"\subset, \Subset, \sqsubset",
            "<math><mrow><mo>⊂</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⋐</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⊏</mo></mrow></math>",
        ),
        (
            r"\supset, \Supset, \sqsupset",
            "<math><mrow><mo>⊃</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⋑</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⊐</mo></mrow></math>",
        ),
        (
            r"\subseteq, \nsubseteq, \subsetneq, \varsubsetneq, \sqsubseteq",
            "<math><mrow><mo>⊆</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⊈</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⊊</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⊊︀</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⊑</mo></mrow></math>",
        ),
        (
            r"\supseteq, \nsupseteq, \supsetneq, \varsupsetneq, \sqsupseteq",
            "<math><mrow><mo>⊇</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⊉</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⊋</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⊋</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⊒</mo></mrow></math>",
        ),
        (
            r"\subseteqq, \nsubseteqq, \subsetneqq, \varsubsetneqq",
            "<math><mrow><mo>⫅</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⊈</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⫋</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⫋︀</mo></mrow></math>",
        ),
        (
            r"\supseteqq, \nsupseteqq, \supsetneqq, \varsupsetneqq",
            "<math><mrow><mo>⫆</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⊉</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⫌</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⫌︀</mo></mrow></math>",
        ),
        (
            r"=, \ne, \neq, \equiv, \not\equiv",
            "<math><mrow><mo>=</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≠</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≠</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≡</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≢</mo></mrow></math>",
        ),
        (
            r"\doteq, \doteqdot, \overset{\underset{\mathrm{def}}{}}{=}, :=",
            "<math><mrow><mo>≐</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≑</mo></mrow><mrow><mo>,</mo></mrow><mrow><mrow><mover><mo>=</mo><mrow><munder><mrow></mrow><mrow><mtext></mtext><mi>def</mi></mrow></munder></mrow></mover></mrow><mo>,</mo></mrow><mrow><mo lspace=\"0.2222em\" rspace=\"0em\">:</mo></mrow><mrow><mo lspace=\"0em\">=</mo></mrow></math>",
        ),
        (
            r"\sim, \nsim, \backsim, \thicksim, \simeq, \backsimeq, \eqsim, \cong, \ncong",
            "<math><mrow><mo>∼</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≁</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>∽</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>∼</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≃</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⋍</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≂</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≅</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≆</mo></mrow></math>",
        ),
        (
            r"\approx, \thickapprox, \approxeq, \asymp, \propto, \varpropto",
            "<math><mrow><mo>≈</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≈</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≊</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≍</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>∝</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>∝</mo></mrow></math>",
        ),
        (
            r"<, \nless, \ll, \not\ll, \lll, \not\lll, \lessdot",
            "<math><mrow><mo>&lt;</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≮</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≪</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≪̸</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⋘</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⋘̸</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">⋖</mo></mrow></math>",
        ),
        (
            r"\le, \leq, \lneq, \leqq, \nleq, \nleqq, \lneqq, \lvertneqq",
            "<math><mrow><mo>≤</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≤</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⪇</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≦</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≰</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≰</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≨</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≨︀</mo></mrow></math>",
        ),
        (
            r"\ge, \geq, \gneq, \geqq, \ngeq, \ngeqq, \gneqq, \gvertneqq",
            "<math><mrow><mo>≥</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≥</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⪈</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≧</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≱</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≱</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≩</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≩︀</mo></mrow></math>",
        ),
        (
            r"\lessgtr, \lesseqgtr, \lesseqqgtr, \gtrless, \gtreqless, \gtreqqless",
            "<math><mrow><mo>≶</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⋚</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⪋</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≷</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⋛</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⪌</mo></mrow></math>",
        ),
        (
            r"\leqslant, \nleqslant, \eqslantless",
            "<math><mrow><mo>⩽</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≰</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⪕</mo></mrow></math>",
        ),
        (
            r"\geqslant, \ngeqslant, \eqslantgtr",
            "<math><mrow><mo>⩾</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>≱</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⪖</mo></mrow></math>",
        ),
        (
            r"\lesssim, \lnsim, \lessapprox, \lnapprox",
            "<math><mrow><mo>≲</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⋦</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⪅</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⪉</mo></mrow></math>",
        ),
        (
            r"\gtrsim, \gnsim, \gtrapprox, \gnapprox",
            "<math><mrow><mo>≳</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⋧</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⪆</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⪊</mo></mrow></math>",
        ),
        (
            r"\prec, \nprec, \preceq, \npreceq, \precneqq",
            "<math><mrow><mo>≺</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⊀</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⪯</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⋠</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⪵</mo></mrow></math>",
        ),
        (
            r"\succ, \nsucc, \succeq, \nsucceq, \succneqq",
            "<math><mrow><mo>≻</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⊁</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⪰</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⋡</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⪶</mo></mrow></math>",
        ),
        (
            r"\preccurlyeq, \curlyeqprec",
            "<math><mrow><mo>≼</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⋞</mo></mrow></math>",
        ),
        (
            r"\succcurlyeq, \curlyeqsucc",
            "<math><mrow><mo>≽</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⋟</mo></mrow></math>",
        ),
        (
            r"\precsim, \precnsim, \precapprox, \precnapprox",
            "<math><mrow><mo>≾</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⋨</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⪷</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⪹</mo></mrow></math>",
        ),
        (
            r"\succsim, \succnsim, \succapprox, \succnapprox",
            "<math><mrow><mo>≿</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⋩</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⪸</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⪺</mo></mrow></math>",
        ),
        (
            r"\parallel, \nparallel, \shortparallel, \nshortparallel",
            "<math><mrow><mo>∥</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>∦</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo mathsize=\"70%\">∥</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo mathsize=\"70%\">∦</mo></mrow></math>",
        ),
        (
            r"\perp, \angle, \sphericalangle, \measuredangle, 45^\circ",
            "<math><mrow><mo>⟂</mo></mrow><mrow><mo>,</mo></mrow><mrow><mi>∠</mi><mo>,</mo></mrow><mrow><mi>∢</mi><mo>,</mo></mrow><mrow><mi>∡</mi><mo>,</mo></mrow><mrow><msup><mn>45</mn><mo>∘</mo></msup></mrow></math>",
        ),
        (
            r"\Box, \square, \blacksquare, \diamond, \Diamond, \lozenge, \blacklozenge,\bigstar",
            "<math><mrow><mi>□</mi><mo>,</mo></mrow><mrow><mi>□</mi><mo>,</mo></mrow><mrow><mi>■</mi><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">⋄</mo></mrow><mrow><mo>,</mo></mrow><mrow><mi>◊</mi><mo>,</mo></mrow><mrow><mi>◊</mi><mo>,</mo></mrow><mrow><mi>⧫</mi><mo>,</mo></mrow><mrow><mi>★</mi></mrow></math>",
        ),
        (
            r"\bigcirc, \triangle, \bigtriangleup, \bigtriangledown",
            "<math><mrow><mo>◯</mo></mrow><mrow><mo>,</mo></mrow><mrow><mi>△</mi><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">△</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">▽</mo></mrow></math>",
        ),
        (
            r"\vartriangle, \triangledown",
            "<math><mrow><mo>△</mo></mrow><mrow><mo>,</mo></mrow><mrow><mi>▽</mi></mrow></math>",
        ),
        (
            r"\blacktriangle, \blacktriangledown, \blacktriangleleft, \blacktriangleright",
            "<math><mrow><mi>▲</mi><mo>,</mo></mrow><mrow><mi>▼</mi><mo>,</mo></mrow><mrow><mo>◀</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>▶</mo></mrow></math>",
        ),
        (
            r"\forall, \exists, \nexists",
            "<math><mrow><mi>∀</mi><mo>,</mo></mrow><mrow><mi>∃</mi><mo>,</mo></mrow><mrow><mi>∄</mi></mrow></math>",
        ),
        (
            r"\therefore, \because, \And",
            "<math><mrow><mo>∴</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>∵</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">&amp;</mo></mrow></math>",
        ),
        (
            r"\lor \vee, \curlyvee, \bigvee",
            "<math><mrow><mo>∨</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">∨</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">⋎</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo movablelimits=\"false\">⋁</mo></mrow></math>",
        ),
        (
            r"\land \wedge, \curlywedge, \bigwedge",
            "<math><mrow><mo>∧</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">∧</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">⋏</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo movablelimits=\"false\">⋀</mo></mrow></math>",
        ),
        (
            r"\bar{q}, \bar{abc}, \overline{q}, \overline{abc}, \\ \lnot \neg, \not\operatorname{R},\bot,\top",
            "<math><mtable columnalign=\"left\" rowspacing=\"0em\"><mtr><mtd style=\"text-align:left;\"><mrow><mover><mi>q</mi><mo stretchy=\"false\" style=\"math-style:normal;math-depth:0;\">‾</mo></mover><mo>,</mo></mrow><mrow><mover><mrow><mi>a</mi><mi>b</mi><mi>c</mi></mrow><mo stretchy=\"false\" style=\"math-style:normal;math-depth:0;\">‾</mo></mover><mo>,</mo></mrow><mrow><mrow style=\"padding:0.1em 0 0 0;border-top:0.065em solid;\"><mi>q</mi></mrow><mo>,</mo></mrow><mrow><mrow style=\"padding:0.1em 0 0 0;border-top:0.065em solid;\"><mrow><mi>a</mi><mi>b</mi><mi>c</mi></mrow></mrow><mo>,</mo></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mo form=\"prefix\" stretchy=\"false\">¬</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">¬</mo></mrow><mrow><mo>,</mo></mrow><mrow><mrow><mi≯</mi><mspace style=\"margin-left:-0.6em;\" width=\"-0.6em\"></mspace><mi mathvariant=\"normal\">R</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mo>,</mo></mrow><mrow><mi>⊥</mi><mo>,</mo></mrow><mrow><mi>⊤</mi></mrow></mtd></mtr></mtable></math>",
        ),
        (
            r"\vdash \dashv, \vDash, \Vdash, \models",
            "<math><mrow><mo rspace=\"0em\">⊢</mo></mrow><mrow><mo lspace=\"0em\">⊣</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⊨</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⊩</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⊨</mo></mrow></math>",
        ),
        (
            r"\Vvdash \nvdash \nVdash \nvDash \nVDash",
            "<math><mrow><mo rspace=\"0em\">⊪</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\">⊬</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\">⊮</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\">⊭</mo></mrow><mrow><mo lspace=\"0em\">⊯</mo></mrow></math>",
        ),
        (
            r"\ulcorner \urcorner \llcorner \lrcorner",
            "<math><mrow><mo form=\"prefix\" stretchy=\"false\">⌜</mo></mrow><mrow><mo form=\"postfix\" stretchy=\"false\">⌝</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">⌞</mo></mrow><mrow><mo form=\"postfix\" stretchy=\"false\">⌟</mo></mrow></math>",
        ),
        (
            r"\Rrightarrow, \Lleftarrow",
            "<math><mrow><mo stretchy=\"false\">⇛</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo stretchy=\"false\">⇚</mo></mrow></math>",
        ),
        (
            r"\Rightarrow, \nRightarrow, \Longrightarrow, \implies",
            "<math><mrow><mo stretchy=\"false\">⇒</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo stretchy=\"false\">⇏</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo stretchy=\"false\">⟹</mo></mrow><mrow><mo>,</mo><mspace width=\"0.2778em\"></mspace></mrow><mrow><mo stretchy=\"false\">⟹</mo><mspace width=\"0.2778em\"></mspace></mrow></math>",
        ),
        (
            r"\Leftarrow, \nLeftarrow, \Longleftarrow",
            "<math><mrow><mo stretchy=\"false\">⇐</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo stretchy=\"false\">⇍</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo stretchy=\"false\">⟸</mo></mrow></math>",
        ),
        (
            r"\Leftrightarrow, \nLeftrightarrow, \Longleftrightarrow, \iff",
            "<math><mrow><mo stretchy=\"false\">⇔</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo stretchy=\"false\">⇎</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo stretchy=\"false\">⟺</mo></mrow><mrow><mo>,</mo><mspace width=\"0.2778em\"></mspace></mrow><mrow><mo stretchy=\"false\">⟺</mo><mspace width=\"0.2778em\"></mspace></mrow></math>",
        ),
        (
            r"\Uparrow, \Downarrow, \Updownarrow",
            "<math><mrow><mo stretchy=\"false\">⇑</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo stretchy=\"false\">⇓</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo stretchy=\"false\">⇕</mo></mrow></math>",
        ),
        (
            r"\rightarrow \to, \nrightarrow, \longrightarrow",
            "<math><mrow><mo rspace=\"0em\" stretchy=\"false\">→</mo></mrow><mrow><mo lspace=\"0em\">→</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo stretchy=\"false\">↛</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo stretchy=\"false\">⟶</mo></mrow></math>",
        ),
        (
            r"\leftarrow \gets, \nleftarrow, \longleftarrow",
            "<math><mrow><mo rspace=\"0em\" stretchy=\"false\">←</mo></mrow><mrow><mo lspace=\"0em\">←</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo stretchy=\"false\">↚</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo stretchy=\"false\">⟵</mo></mrow></math>",
        ),
        (
            r"\leftrightarrow, \nleftrightarrow, \longleftrightarrow",
            "<math><mrow><mo stretchy=\"false\">↔</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo stretchy=\"false\">↮</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo stretchy=\"false\">⟷</mo></mrow></math>",
        ),
        (
            r"\uparrow, \downarrow, \updownarrow",
            "<math><mrow><mo stretchy=\"false\">↑</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo stretchy=\"false\">↓</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo stretchy=\"false\">↕</mo></mrow></math>",
        ),
        (
            r"\nearrow, \swarrow, \nwarrow, \searrow",
            "<math><mrow><mo stretchy=\"false\">↗</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo stretchy=\"false\">↙</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo stretchy=\"false\">↖</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo stretchy=\"false\">↘</mo></mrow></math>",
        ),
        (
            r"\mapsto, \longmapsto",
            "<math><mrow><mo>↦</mo></mrow><mrow><mo>,</mo></mrow><mrow><mo>⟼</mo></mrow></math>",
        ),
        (
            r"\rightharpoonup \rightharpoondown \leftharpoonup \leftharpoondown \upharpoonleft \upharpoonright \downharpoonleft \downharpoonright \rightleftharpoons \leftrightharpoons",
            "<math><mrow><mo rspace=\"0em\" stretchy=\"false\">⇀</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">⇁</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">↼</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">↽</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">↿</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">↾</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">⇃</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">⇂</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">⇌</mo></mrow><mrow><mo lspace=\"0em\" stretchy=\"false\">⇋</mo></mrow></math>",
        ),
        (
            r"\curvearrowleft \circlearrowleft \Lsh \upuparrows \rightrightarrows \rightleftarrows \rightarrowtail \looparrowright",
            "<math><mrow><mo rspace=\"0em\" stretchy=\"false\">↶</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">↺</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">↰</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">⇈</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">⇉</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">⇄</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">↣</mo></mrow><mrow><mo lspace=\"0em\" stretchy=\"false\">↬</mo></mrow></math>",
        ),
        (
            r"\curvearrowright \circlearrowright \Rsh \downdownarrows \leftleftarrows \leftrightarrows \leftarrowtail \looparrowleft",
            "<math><mrow><mo rspace=\"0em\" stretchy=\"false\">↷</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">↻</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">↱</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">⇊</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">⇇</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">⇆</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">↢</mo></mrow><mrow><mo lspace=\"0em\" stretchy=\"false\">↫</mo></mrow></math>",
        ),
        (
            r"\hookrightarrow \hookleftarrow \multimap \leftrightsquigarrow \rightsquigarrow \twoheadrightarrow \twoheadleftarrow",
            "<math><mrow><mo rspace=\"0em\" stretchy=\"false\">↪</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">↩</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\">⊸</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">↭</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">⇝</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">↠</mo></mrow><mrow><mo lspace=\"0em\" stretchy=\"false\">↞</mo></mrow></math>",
        ),
        (
            r"\amalg \P \S \% \dagger\ddagger\ldots\cdots",
            "<math><mrow><mo>⨿</mo></mrow><mrow><mi>¶</mi><mi>§</mi><mi>%</mi><mo>†</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">‡</mo></mrow><mrow><mo>…</mo></mrow><mrow><mo>⋯</mo></mrow></math>",
        ),
        (
            r"\smile \frown \wr \triangleleft \triangleright",
            "<math><mrow><mo rspace=\"0em\">⌣</mo></mrow><mrow><mo lspace=\"0em\">⌢</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">≀</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">◃</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">▹</mo></mrow></math>",
        ),
        (
            r"\diamondsuit, \heartsuit, \clubsuit, \spadesuit, \Game, \flat, \natural, \sharp",
            "<math><mrow><mi>♢</mi><mo>,</mo></mrow><mrow><mi>♡</mi><mo>,</mo></mrow><mrow><mi>♣</mi><mo>,</mo></mrow><mrow><mi>♠</mi><mo>,</mo></mrow><mrow><mi>⅁</mi><mo>,</mo></mrow><mrow><mi>♭</mi><mo>,</mo></mrow><mrow><mi>♮</mi><mo>,</mo></mrow><mrow><mi>♯</mi></mrow></math>",
        ),
        (
            r"\diagup \diagdown \centerdot \ltimes \rtimes \leftthreetimes \rightthreetimes",
            "<math><mrow><mi>╱</mi><mi>╲</mi><mrow><mspace width=\"0.2222em\"></mspace><mspace height=\"0.189em\" mathbackground=\"black\" width=\"0.167em\"></mspace><mspace width=\"0.2222em\"></mspace></mrow><mo>⋉</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">⋊</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">⋋</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">⋌</mo></mrow></math>",
        ),
        (
            r"\eqcirc \circeq \triangleq \bumpeq\Bumpeq \doteqdot \risingdotseq \fallingdotseq",
            "<math><mrow><mo rspace=\"0em\">≖</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\">≗</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\">≜</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\">≏</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\">≎</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\">≑</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\">≓</mo></mrow><mrow><mo lspace=\"0em\">≒</mo></mrow></math>",
        ),
        (
            r"\intercal \barwedge \veebar \doublebarwedge \between \pitchfork",
            "<math><mrow><mo>⊺</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">⊼</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">⊻</mo></mrow><mrow><mo form=\"prefix\" stretchy=\"false\">⩞</mo></mrow><mrow><mo rspace=\"0em\">≬</mo></mrow><mrow><mo lspace=\"0em\">⋔</mo></mrow></math>",
        ),
        (
            r"\vartriangleleft \ntriangleleft \vartriangleright \ntriangleright",
            "<math><mrow><mo rspace=\"0em\">⊲</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\">⋪</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\">⊳</mo></mrow><mrow><mo lspace=\"0em\">⋫</mo></mrow></math>",
        ),
        (
            r"\trianglelefteq \ntrianglelefteq \trianglerighteq \ntrianglerighteq",
            "<math><mrow><mo rspace=\"0em\">⊴</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\">⋬</mo></mrow><mrow><mo lspace=\"0em\" rspace=\"0em\">⊵</mo></mrow><mrow><mo lspace=\"0em\">⋭</mo></mrow></math>",
        ),
        (
            r"a^2, a^{x+3}",
            "<math><mrow><msup><mi>a</mi><mn>2</mn></msup><mo>,</mo></mrow><mrow><msup><mi>a</mi><mrow><mi>x</mi><mo>+</mo><mn>3</mn></mrow></msup></mrow></math>",
        ),
        (r"a_2", "<math><msub><mi>a</mi><mn>2</mn></msub></math>"),
        (
            r"10^{30} a^{2+2} \\ a_{i,j} b_{f'}",
            "<math><mtable columnalign=\"left\" rowspacing=\"0em\"><mtr><mtd style=\"text-align:left;\"><mrow><msup><mn>10</mn><mn>30</mn></msup><msup><mi>a</mi><mrow><mn>2</mn><mo>+</mo><mn>2</mn></mrow></msup></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><msub><mi>a</mi><mrow><mi>i</mi><mo>,</mo><mi>j</mi></mrow></msub><msub><mi>b</mi><msup><mi>f</mi><mo class=\"tml-prime prime-pad\" lspace=\"0em\" rspace=\"0em\">′</mo></msup></msub></mrow></mtd></mtr></mtable></math>",
        ),
        (
            r"x_2^3 \\ {x_2}^3",
            "<math><mtable columnalign=\"left\" rowspacing=\"0em\"><mtr><mtd style=\"text-align:left;\"><mrow><msubsup><mi>x</mi><mn>2</mn><mn>3</mn></msubsup></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><msup><msub><mi>x</mi><mn>2</mn></msub><mn>3</mn></msup></mrow></mtd></mtr></mtable></math>",
        ),
        (
            r"10^{10^{8}}",
            "<math><msup><mn>10</mn><msup><mn>10</mn><mn>8</mn></msup></msup></math>",
        ),
        (
            r"\overset{\alpha}{\omega} \\ \underset{\alpha}{\omega} \\ \overset{\alpha}{\underset{\gamma}{\omega}}\\ \stackrel{\alpha}{\omega}",
            "<math><mtable columnalign=\"left\" rowspacing=\"0em\"><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mover><mi>ω</mi><mi>α</mi></mover></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow><munder><mi>ω</mi><mi>α</mi></munder></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mover><mrow><munder><mi>ω</mi><mi>γ</mi></munder></mrow><mi>α</mi></mover></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mover><mi>ω</mi><mi>α</mi></mover></mrow></mrow></mtd></mtr></mtable></math>",
        ),
        (
            r"x', y'', f', f'' \\ x^\prime, y^{\prime\prime}",
            "<math><mtable columnalign=\"left\" rowspacing=\"0em\"><mtr><mtd style=\"text-align:left;\"><mrow><msup><mi>x</mi><mo lspace=\"0em\" rspace=\"0em\">′</mo></msup><mo>,</mo></mrow><mrow><msup><mi>y</mi><mrow><mo lspace=\"0em\" rspace=\"0em\">′</mo><mo lspace=\"0em\" rspace=\"0em\">′</mo></mrow></msup><mo>,</mo></mrow><mrow><msup><mi>f</mi><mo class=\"tml-prime prime-pad\" lspace=\"0em\" rspace=\"0em\">′</mo></msup><mo>,</mo></mrow><mrow><msup><mi>f</mi><mrow><mo class=\"tml-prime prime-pad\" lspace=\"0em\" rspace=\"0em\">′</mo><mo lspace=\"0em\" rspace=\"0em\">′</mo></mrow></msup></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><msup><mi>x</mi><mo>′</mo></msup><mo>,</mo></mrow><mrow><msup><mi>y</mi><mrow><mo lspace=\"0em\" rspace=\"0em\">′</mo><mo lspace=\"0em\" rspace=\"0em\">′</mo></mrow></msup></mrow></mtd></mtr></mtable></math>",
        ),
        (
            r"\dot{x}, \ddot{x}",
            "<math><mrow><mover><mi>x</mi><mo stretchy=\"false\" style=\"math-style:normal;math-depth:0;\">˙</mo></mover><mo>,</mo></mrow><mrow><mover><mi>x</mi><mo stretchy=\"false\" style=\"math-style:normal;math-depth:0;\">¨</mo></mover></mrow></math>",
        ),
        (
            r"\hat a \ \bar b \ \vec c \\ \overrightarrow{a b} \ \overleftarrow{c d}\\ \widehat{d e f} \\ \overline{g h i} \ \underline{j k l}",
            "<math><mtable columnalign=\"left\" rowspacing=\"0em\"><mtr><mtd style=\"text-align:left;\"><mrow><mover><mi>a</mi><mo stretchy=\"false\" style=\"math-style:normal;math-depth:0;\">^</mo></mover><mtext> </mtext><mover><mi>b</mi><mo class=\"tml-capshift\" stretchy=\"false\" style=\"math-style:normal;math-depth:0;\">‾</mo></mover><mtext> </mtext><mover><mi>c</mi><mo stretchy=\"false\" style=\"transform:scale(0.75) translate(10%, 30%);\">→</mo></mover></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mover><mrow><mi>a</mi><mi>b</mi></mrow><mo stretchy=\"true\" style=\"math-style:normal;math-depth:0;\">→</mo></mover><mtext> </mtext><mover><mrow><mi>c</mi><mi>d</mi></mrow><mo stretchy=\"true\" style=\"math-style:normal;math-depth:0;\">←</mo></mover></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mover><mrow><mi>d</mi><mi>e</mi><mi>f</mi></mrow><mo class=\"tml-crooked-3\" stretchy=\"true\" style=\"math-style:normal;math-depth:0;\">^</mo></mover></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow style=\"padding:0.1em 0 0 0;border-top:0.065em solid;\"><mrow><mi>g</mi><mi>h</mi><mi>i</mi></mrow></mrow><mtext> </mtext><mrow style=\"padding:0 0 0.1em 0;border-bottom:0.065em solid;\"><mrow><mi>j</mi><mi>k</mi><mi>l</mi></mrow></mrow></mrow></mtd></mtr></mtable></math>",
        ),
        (
            r"\overset{\frown} {AB}",
            "<math><mrow><mover><mi>A</mi><mo lspace=\"0em\" rspace=\"0em\">⌢</mo></mover></mrow></math>",
        ),
        (
            r"A \xleftarrow{n+\mu-1} B \xrightarrow[T]{n\pm i-1} C",
            "<math><mrow><mi>A</mi><mrow><mspace width=\"0.2778em\"></mspace><mover accent=\"false\"><mo lspace=\"0\" rspace=\"0\" stretchy=\"true\">←</mo><mover><mrow><mspace width=\"0.4286em\"></mspace><mrow><mi>n</mi><mo>+</mo><mi>μ</mi><mo>−</mo><mn>1</mn></mrow><mspace width=\"0.4286em\"></mspace></mrow><mspace width=\"3.5000em\"></mspace></mover></mover><mspace width=\"0.2778em\"></mspace></mrow><mi>B</mi><mrow><mspace width=\"0.2778em\"></mspace><munderover accent=\"false\"><mo lspace=\"0\" rspace=\"0\" stretchy=\"true\">→</mo><munder><mrow><mspace width=\"0.4286em\"></mspace><mi>T</mi><mspace width=\"0.4286em\"></mspace></mrow><mspace width=\"3.5000em\"></mspace></munder><mover><mrow><mspace width=\"0.4286em\"></mspace><mrow><mi>n</mi><mo>±</mo><mi>i</mi><mo>−</mo><mn>1</mn></mrow><mspace width=\"0.4286em\"></mspace></mrow><mspace width=\"3.5000em\"></mspace></mover></munderover><mspace width=\"0.2778em\"></mspace></mrow><mi>C</mi></mrow></math>",
        ),
        (
            r"\overbrace{ 1+2+\cdots+100 }^{5050}",
            "<math><mrow><mover><mover><mrow><mn>1</mn><mo>+</mo><mn>2</mn><mo>+</mo><mo>⋯</mo><mo>+</mo><mn>100</mn></mrow><mo stretchy=\"true\" style=\"math-depth:0;\">⏞</mo></mover><mn>5050</mn></mover></mrow></math>",
        ),
        (
            r"\underbrace{ a+b+\cdots+z }_{26}",
            "<math><mrow><munder><munder><mrow><mi>a</mi><mo>+</mo><mi>b</mi><mo>+</mo><mo>⋯</mo><mo>+</mo><mi>z</mi></mrow><mo stretchy=\"true\" style=\"math-depth:0;\">⏟</mo></munder><mn>26</mn></munder></mrow></math>",
        ),
        (
            r"\frac{2}{4}=0.5",
            "<math><mrow><mfrac><mn>2</mn><mn>4</mn></mfrac><mo>=</mo></mrow><mrow><mn>0.5</mn></mrow></math>",
        ),
        (
            r"\dfrac{2}{4} = 0.5 \qquad \dfrac{2}{c + \dfrac{2}{d + \dfrac{2}{4}}} = a",
            "<math><mrow><mstyle displaystyle=\"true\" scriptlevel=\"0\"><mfrac><mn>2</mn><mn>4</mn></mfrac></mstyle><mo>=</mo></mrow><mrow><mn>0.5</mn><mspace width=\"2em\"></mspace><mstyle displaystyle=\"true\" scriptlevel=\"0\"><mfrac><mn>2</mn><mrow><mi>c</mi><mo>+</mo><mstyle displaystyle=\"true\" scriptlevel=\"0\"><mfrac><mn>2</mn><mrow><mi>d</mi><mo>+</mo><mstyle displaystyle=\"true\" scriptlevel=\"0\"><mfrac><mn>2</mn><mn>4</mn></mfrac></mstyle></mrow></mfrac></mstyle></mrow></mfrac></mstyle><mo>=</mo></mrow><mrow><mi>a</mi></mrow></math>",
        ),
        (
            r"\cfrac{x}{1 + \cfrac{\cancel{y}} {\cancel{y}}} = \cfrac{x}{2}",
            "<math><mrow><mstyle displaystyle=\"true\" scriptlevel=\"0\"><mfrac><mi>x</mi><mrow><mn>1</mn><mo>+</mo><mstyle displaystyle=\"true\" scriptlevel=\"0\"><mfrac><mrow class=\"tml-cancel\"><mi>y</mi></mrow><mrow class=\"tml-cancel\"><mi>y</mi></mrow></mfrac></mstyle></mrow></mfrac></mstyle><mo>=</mo></mrow><mrow><mstyle displaystyle=\"true\" scriptlevel=\"0\"><mfrac><mi>x</mi><mn>2</mn></mfrac></mstyle></mrow></math>",
        ),
        (
            r"\binom{n}{k}",
            "<math><mrow><mo fence=\"true\">(</mo><mfrac linethickness=\"0px\"><mi>n</mi><mi>k</mi></mfrac><mo fence=\"true\">)</mo></mrow></math>",
        ),
        (
            r"\dbinom{n}{k}",
            "<math><mstyle displaystyle=\"true\" scriptlevel=\"0\"><mrow><mo fence=\"true\">(</mo><mfrac linethickness=\"0px\"><mi>n</mi><mi>k</mi></mfrac><mo fence=\"true\">)</mo></mrow></mstyle></math>",
        ),
        (
            r"\begin{matrix} x & y \\ z & v \end{matrix}",
            "<math><mtable columnalign=\"center center\"><mtr><mtd style=\"padding-left:0em;\"><mi>x</mi></mtd><mtd style=\"padding-right:0em;\"><mi>y</mi></mtd></mtr><mtr><mtd style=\"padding-left:0em;\"><mi>z</mi></mtd><mtd style=\"padding-right:0em;\"><mi>v</mi></mtd></mtr></mtable></math>",
        ),
        (
            r"\begin{vmatrix} x & y \\ z & v \end{vmatrix}",
            "<math><mrow><mo fence=\"true\" form=\"prefix\">|</mo><mtable columnalign=\"center center\"><mtr><mtd style=\"padding-left:0em;\"><mi>x</mi></mtd><mtd style=\"padding-right:0em;\"><mi>y</mi></mtd></mtr><mtr><mtd style=\"padding-left:0em;\"><mi>z</mi></mtd><mtd style=\"padding-right:0em;\"><mi>v</mi></mtd></mtr></mtable><mo fence=\"true\" form=\"postfix\">|</mo></mrow></math>",
        ),
        (
            r"\begin{Vmatrix} x & y \\ z & v \end{Vmatrix}",
            "<math><mrow><mo fence=\"true\" form=\"prefix\">‖</mo><mtable columnalign=\"center center\"><mtr><mtd style=\"padding-left:0em;\"><mi>x</mi></mtd><mtd style=\"padding-right:0em;\"><mi>y</mi></mtd></mtr><mtr><mtd style=\"padding-left:0em;\"><mi>z</mi></mtd><mtd style=\"padding-right:0em;\"><mi>v</mi></mtd></mtr></mtable><mo fence=\"true\" form=\"postfix\">‖</mo></mrow></math>",
        ),
        (
            r"\begin{bmatrix} 0 & \cdots & 0 \\ \vdots & \ddots & \vdots \\ 0 & \cdots & 0 \end{bmatrix}",
            "<math><mrow><mo fence=\"true\" form=\"prefix\">[</mo><mtable columnalign=\"center center center\"><mtr><mtd style=\"padding-left:0em;\"><mn>0</mn></mtd><mtd><mo>⋯</mo></mtd><mtd style=\"padding-right:0em;\"><mn>0</mn></mtd></mtr><mtr><mtd style=\"padding-left:0em;\"><mrow><mi>⋮</mi><mspace height=\"14.944pt\" width=\"0pt\"></mspace></mrow></mtd><mtd><mo>⋱</mo></mtd><mtd style=\"padding-right:0em;\"><mrow><mi>⋮</mi><mspace height=\"14.944pt\" width=\"0pt\"></mspace></mrow></mtd></mtr><mtr><mtd style=\"padding-left:0em;\"><mn>0</mn></mtd><mtd><mo>⋯</mo></mtd><mtd style=\"padding-right:0em;\"><mn>0</mn></mtd></mtr></mtable><mo fence=\"true\" form=\"postfix\">]</mo></mrow></math>",
        ),
        (
            r"\begin{Bmatrix} x & y \\ z & v \end{Bmatrix}",
            "<math><mrow><mo fence=\"true\" form=\"prefix\">{</mo><mtable columnalign=\"center center\"><mtr><mtd style=\"padding-left:0em;\"><mi>x</mi></mtd><mtd style=\"padding-right:0em;\"><mi>y</mi></mtd></mtr><mtr><mtd style=\"padding-left:0em;\"><mi>z</mi></mtd><mtd style=\"padding-right:0em;\"><mi>v</mi></mtd></mtr></mtable><mo fence=\"true\" form=\"postfix\">}</mo></mrow></math>",
        ),
        (
            r"\begin{pmatrix} x & y \\ z & v \end{pmatrix}",
            "<math><mrow><mo fence=\"true\" form=\"prefix\">(</mo><mtable columnalign=\"center center\"><mtr><mtd style=\"padding-left:0em;\"><mi>x</mi></mtd><mtd style=\"padding-right:0em;\"><mi>y</mi></mtd></mtr><mtr><mtd style=\"padding-left:0em;\"><mi>z</mi></mtd><mtd style=\"padding-right:0em;\"><mi>v</mi></mtd></mtr></mtable><mo fence=\"true\" form=\"postfix\">)</mo></mrow></math>",
        ),
        (
            r"\bigl( \begin{smallmatrix} a&b\\ c&d \end{smallmatrix} \bigr)",
            "<math><mrow><mo fence=\"true\" maxsize=\"1.2em\" minsize=\"1.2em\" symmetric=\"true\">(</mo><mstyle scriptlevel=\"1\"><mtable><mtr><mtd style=\"padding:0.35ex 0.1389em 0.35ex 0em;\"><mi>a</mi></mtd><mtd style=\"padding:0.35ex 0em 0.35ex 0.1389em;\"><mi>b</mi></mtd></mtr><mtr><mtd style=\"padding:0.35ex 0.1389em 0.35ex 0em;\"><mi>c</mi></mtd><mtd style=\"padding:0.35ex 0em 0.35ex 0.1389em;\"><mi>d</mi></mtd></mtr></mtable></mstyle><mo fence=\"true\" maxsize=\"1.2em\" minsize=\"1.2em\" symmetric=\"true\">)</mo></mrow></math>",
        ),
        (
            r"f(n) = \begin{cases} n/2, & \text{if }n\text{ is even} \\ 3n+1, & \text{if }n\text{ is odd} \end{cases}",
            "<math><mrow><mi>f</mi><mo form=\"prefix\" stretchy=\"false\">(</mo><mi>n</mi><mo form=\"postfix\" stretchy=\"false\">)</mo><mo>=</mo></mrow><mrow><mrow><mo fence=\"true\" form=\"prefix\">{</mo><mtable><mtr><mtd class=\"tml-left\" style=\"padding:0.5ex 0em 0.5ex 0em;\"><mrow><mi>n</mi><mi>/</mi><mn>2</mn><mo>,</mo></mrow></mtd><mtd class=\"tml-left\" style=\"padding:0.5ex 0em 0.5ex 1em;\"><mrow><mtext>if </mtext><mi>n</mi><mtext> is even</mtext></mrow></mtd></mtr><mtr><mtd class=\"tml-left\" style=\"padding:0.5ex 0em 0.5ex 0em;\"><mrow><mn>3</mn><mi>n</mi><mo>+</mo><mn>1</mn><mo>,</mo></mrow></mtd><mtd class=\"tml-left\" style=\"padding:0.5ex 0em 0.5ex 1em;\"><mrow><mtext>if </mtext><mi>n</mi><mtext> is odd</mtext></mrow></mtd></mtr></mtable><mo fence=\"true\" form=\"postfix\"></mo></mrow></mrow></math>",
        ),
        (
            r"\begin{cases} 3x + 5y + z \\ 7x - 2y + 4z \\ -6x + 3y + 2z \end{cases}",
            "<math><mrow><mo fence=\"true\" form=\"prefix\">{</mo><mtable><mtr><mtd class=\"tml-left\" style=\"padding:0.5ex 0em 0.5ex 0em;\"><mrow><mn>3</mn><mi>x</mi><mo>+</mo><mn>5</mn><mi>y</mi><mo>+</mo><mi>z</mi></mrow></mtd></mtr><mtr><mtd class=\"tml-left\" style=\"padding:0.5ex 0em 0.5ex 0em;\"><mrow><mn>7</mn><mi>x</mi><mo>−</mo><mn>2</mn><mi>y</mi><mo>+</mo><mn>4</mn><mi>z</mi></mrow></mtd></mtr><mtr><mtd class=\"tml-left\" style=\"padding:0.5ex 0em 0.5ex 0em;\"><mrow><mo>−</mo><mn>6</mn><mi>x</mi><mo>+</mo><mn>3</mn><mi>y</mi><mo>+</mo><mn>2</mn><mi>z</mi></mrow></mtd></mtr></mtable><mo fence=\"true\" form=\"postfix\"></mo></mrow></math>",
        ),
        (
            r"f(x) \,\!",
            "<math><mrow><mi>f</mi><mo form=\"prefix\" stretchy=\"false\">(</mo><mi>x</mi><mo form=\"postfix\" stretchy=\"false\">)</mo><mspace width=\"0.1667em\"></mspace><mspace style=\"margin-left:-0.1667em;\" width=\"-0.1667em\"></mspace></mrow></math>",
        ),
        (
            r"\begin{array}{|c|c|c|} a & b & S \\ \hline 0 & 0 & 1 \\ 0 & 1 & 1 \\ 1 & 0 & 1 \\ 1 & 1 & 0 \\ \end{array}",
            "<math><mtable columnalign=\"center center center\"><mtr><mtd style=\"border-bottom:0.06em solid;padding:0.5ex 5.9776pt 0.5ex 0pt;border-left:0.06em solid ;border-right:0.06em solid;\"><mi>a</mi></mtd><mtd style=\"border-bottom:0.06em solid;padding:0.5ex 5.9776pt 0.5ex 5.9776pt;border-right:0.06em solid;\"><mi>b</mi></mtd><mtd style=\"border-bottom:0.06em solid;padding:0.5ex 0pt 0.5ex 5.9776pt;border-right:0.06em solid;padding-right:0.4em;\"><mi>S</mi></mtd></mtr><mtr><mtd style=\"padding:0.5ex 5.9776pt 0.5ex 0pt;border-left:0.06em solid ;border-right:0.06em solid;\"><mn>0</mn></mtd><mtd style=\"padding:0.5ex 5.9776pt 0.5ex 5.9776pt;border-right:0.06em solid;\"><mn>0</mn></mtd><mtd style=\"padding:0.5ex 0pt 0.5ex 5.9776pt;border-right:0.06em solid;padding-right:0.4em;\"><mn>1</mn></mtd></mtr><mtr><mtd style=\"padding:0.5ex 5.9776pt 0.5ex 0pt;border-left:0.06em solid ;border-right:0.06em solid;\"><mn>0</mn></mtd><mtd style=\"padding:0.5ex 5.9776pt 0.5ex 5.9776pt;border-right:0.06em solid;\"><mn>1</mn></mtd><mtd style=\"padding:0.5ex 0pt 0.5ex 5.9776pt;border-right:0.06em solid;padding-right:0.4em;\"><mn>1</mn></mtd></mtr><mtr><mtd style=\"padding:0.5ex 5.9776pt 0.5ex 0pt;border-left:0.06em solid ;border-right:0.06em solid;\"><mn>1</mn></mtd><mtd style=\"padding:0.5ex 5.9776pt 0.5ex 5.9776pt;border-right:0.06em solid;\"><mn>0</mn></mtd><mtd style=\"padding:0.5ex 0pt 0.5ex 5.9776pt;border-right:0.06em solid;padding-right:0.4em;\"><mn>1</mn></mtd></mtr><mtr><mtd style=\"padding:0.5ex 5.9776pt 0.5ex 0pt;border-left:0.06em solid ;border-right:0.06em solid;\"><mn>1</mn></mtd><mtd style=\"padding:0.5ex 5.9776pt 0.5ex 5.9776pt;border-right:0.06em solid;\"><mn>1</mn></mtd><mtd style=\"padding:0.5ex 0pt 0.5ex 5.9776pt;border-right:0.06em solid;padding-right:0.4em;\"><mn>0</mn></mtd></mtr></mtable></math>",
        ),
        (
            r"( \frac{1}{2} )^n",
            "<math><mrow><mo form=\"prefix\" stretchy=\"false\">(</mo><mfrac><mn>1</mn><mn>2</mn></mfrac><msup><mo form=\"postfix\" stretchy=\"false\">)</mo><mi>n</mi></msup></mrow></math>",
        ),
        (
            r"\left ( \frac{1}{2} \right )^n",
            "<math><msup><mrow><mo fence=\"true\" form=\"prefix\">(</mo><mfrac><mn>1</mn><mn>2</mn></mfrac><mo fence=\"true\" form=\"postfix\">)</mo></mrow><mi>n</mi></msup></math>",
        ),
        (
            r"\left ( \frac{a}{b} \right )",
            "<math><mrow><mo fence=\"true\" form=\"prefix\">(</mo><mfrac><mi>a</mi><mi>b</mi></mfrac><mo fence=\"true\" form=\"postfix\">)</mo></mrow></math>",
        ),
        (
            r"\left [ \frac{a}{b} \right ] \quad \left \lbrack \frac{a}{b} \right \rbrack",
            "<math><mrow><mrow><mo fence=\"true\" form=\"prefix\">[</mo><mfrac><mi>a</mi><mi>b</mi></mfrac><mo fence=\"true\" form=\"postfix\">]</mo></mrow><mspace width=\"1em\"></mspace><mrow><mo fence=\"true\" form=\"prefix\">[</mo><mfrac><mi>a</mi><mi>b</mi></mfrac><mo fence=\"true\" form=\"postfix\">]</mo></mrow></mrow></math>",
        ),
        (
            r"\left \{ \frac{a}{b} \right \} \quad \left \lbrace \frac{a}{b} \right \rbrace",
            "<math><mrow><mrow><mo fence=\"true\" form=\"prefix\">{</mo><mfrac><mi>a</mi><mi>b</mi></mfrac><mo fence=\"true\" form=\"postfix\">}</mo></mrow><mspace width=\"1em\"></mspace><mrow><mo fence=\"true\" form=\"prefix\">{</mo><mfrac><mi>a</mi><mi>b</mi></mfrac><mo fence=\"true\" form=\"postfix\">}</mo></mrow></mrow></math>",
        ),
        (
            r"\left \langle \frac{a}{b} \right \rangle",
            "<math><mrow><mo fence=\"true\" form=\"prefix\">⟨</mo><mfrac><mi>a</mi><mi>b</mi></mfrac><mo fence=\"true\" form=\"postfix\">⟩</mo></mrow></math>",
        ),
        (
            r"\left | \frac{a}{b} \right \vert \quad \left \Vert \frac{c}{d} \right \|",
            "<math><mrow><mrow><mo fence=\"true\" form=\"prefix\">|</mo><mfrac><mi>a</mi><mi>b</mi></mfrac><mo fence=\"true\" form=\"postfix\">|</mo></mrow><mspace width=\"1em\"></mspace><mrow><mo fence=\"true\" form=\"prefix\">‖</mo><mfrac><mi>c</mi><mi>d</mi></mfrac><mo fence=\"true\" form=\"postfix\">‖</mo></mrow></mrow></math>",
        ),
        (
            r"\left \lfloor \frac{a}{b} \right \rfloor \quad \left \lceil \frac{c}{d} \right \rceil",
            "<math><mrow><mrow><mo fence=\"true\" form=\"prefix\">⌊</mo><mfrac><mi>a</mi><mi>b</mi></mfrac><mo fence=\"true\" form=\"postfix\">⌋</mo></mrow><mspace width=\"1em\"></mspace><mrow><mo fence=\"true\" form=\"prefix\">⌈</mo><mfrac><mi>c</mi><mi>d</mi></mfrac><mo fence=\"true\" form=\"postfix\">⌉</mo></mrow></mrow></math>",
        ),
        (
            r"\left / \frac{a}{b} \right \backslash",
            "<math><mrow><mo fence=\"true\" form=\"prefix\">∕</mo><mfrac><mi>a</mi><mi>b</mi></mfrac><mo fence=\"true\" form=\"postfix\" stretchy=\"true\">∖</mo></mrow></math>",
        ),
        (
            r"\left\uparrow\frac{a}{b}\right\downarrow\; \left\Uparrow\frac{a}{b}\right\Downarrow\; \left \updownarrow \frac{a}{b} \right \Updownarrow",
            "<math><mrow><mrow><mo fence=\"true\" form=\"prefix\" stretchy=\"true\">↑</mo><mfrac><mi>a</mi><mi>b</mi></mfrac><mo fence=\"true\" form=\"postfix\" stretchy=\"true\">↓</mo></mrow><mspace width=\"0.2778em\"></mspace><mrow><mo fence=\"true\" form=\"prefix\" stretchy=\"true\">⇑</mo><mfrac><mi>a</mi><mi>b</mi></mfrac><mo fence=\"true\" form=\"postfix\" stretchy=\"true\">⇓</mo></mrow><mspace width=\"0.2778em\"></mspace><mrow><mo fence=\"true\" form=\"prefix\" stretchy=\"true\">↕</mo><mfrac><mi>a</mi><mi>b</mi></mfrac><mo fence=\"true\" form=\"postfix\" stretchy=\"true\">⇕</mo></mrow></mrow></math>",
        ),
        (
            r"\left [ 0,1 \right ) \left \langle \psi \right |",
            "<math><mrow><mrow><mo fence=\"true\" form=\"prefix\">[</mo><mn>0,1</mn><mo fence=\"true\" form=\"postfix\">)</mo></mrow><mrow><mo fence=\"true\" form=\"prefix\">⟨</mo><mi>ψ</mi><mo fence=\"true\" form=\"postfix\">|</mo></mrow></mrow></math>",
        ),
        (
            r"\left . \frac{A}{B} \right \} \to X",
            "<math><mrow><mrow><mo fence=\"true\" form=\"prefix\"></mo><mfrac><mi>A</mi><mi>B</mi></mfrac><mo fence=\"true\" form=\"postfix\">}</mo></mrow><mo>→</mo></mrow><mrow><mi>X</mi></mrow></math>",
        ),
        (
            r"( \bigl( \Bigl( \biggl( \Biggl( \dots \Biggr] \biggr] \Bigr] \bigr] ]",
            "<math><mrow><mo form=\"prefix\" stretchy=\"false\">(</mo><mo fence=\"true\" maxsize=\"1.2em\" minsize=\"1.2em\" symmetric=\"true\">(</mo><mo fence=\"true\" maxsize=\"1.8em\" minsize=\"1.8em\" symmetric=\"true\">(</mo><mo fence=\"true\" maxsize=\"2.4em\" minsize=\"2.4em\" symmetric=\"true\">(</mo><mo fence=\"true\" maxsize=\"3em\" minsize=\"3em\" symmetric=\"true\">(</mo><mo>…</mo><mspace width=\"0.1667em\"></mspace><mo fence=\"true\" maxsize=\"3em\" minsize=\"3em\" symmetric=\"true\">]</mo><mo fence=\"true\" maxsize=\"2.4em\" minsize=\"2.4em\" symmetric=\"true\">]</mo><mo fence=\"true\" maxsize=\"1.8em\" minsize=\"1.8em\" symmetric=\"true\">]</mo><mo fence=\"true\" maxsize=\"1.2em\" minsize=\"1.2em\" symmetric=\"true\">]</mo><mo form=\"postfix\" stretchy=\"false\">]</mo></mrow></math>",
        ),
        (
            r"\{ \bigl\{ \Bigl\{ \biggl\{ \Biggl\{ \dots \Biggr\rangle \biggr\rangle \Bigr\rangle \bigr\rangle \rangle",
            "<math><mrow><mo form=\"prefix\" stretchy=\"false\">{</mo><mo fence=\"true\" maxsize=\"1.2em\" minsize=\"1.2em\" symmetric=\"true\">{</mo><mo fence=\"true\" maxsize=\"1.8em\" minsize=\"1.8em\" symmetric=\"true\">{</mo><mo fence=\"true\" maxsize=\"2.4em\" minsize=\"2.4em\" symmetric=\"true\">{</mo><mo fence=\"true\" maxsize=\"3em\" minsize=\"3em\" symmetric=\"true\">{</mo><mo>…</mo><mspace width=\"0.1667em\"></mspace><mo fence=\"true\" maxsize=\"3em\" minsize=\"3em\" symmetric=\"true\">⟩</mo><mo fence=\"true\" maxsize=\"2.4em\" minsize=\"2.4em\" symmetric=\"true\">⟩</mo><mo fence=\"true\" maxsize=\"1.8em\" minsize=\"1.8em\" symmetric=\"true\">⟩</mo><mo fence=\"true\" maxsize=\"1.2em\" minsize=\"1.2em\" symmetric=\"true\">⟩</mo><mo form=\"postfix\" stretchy=\"false\">⟩</mo></mrow></math>",
        ),
        (
            r"\| \big\| \Big\| \bigg\| \Bigg\| \dots \Bigg| \bigg| \Big| \big| |",
            "<math><mrow><mi>‖</mi><mo fence=\"false\" maxsize=\"1.2em\" minsize=\"1.2em\" symmetric=\"true\">‖</mo></mrow><mrow><mo fence=\"false\" maxsize=\"1.8em\" minsize=\"1.8em\" symmetric=\"true\">‖</mo></mrow><mrow><mo fence=\"false\" maxsize=\"2.4em\" minsize=\"2.4em\" symmetric=\"true\">‖</mo></mrow><mrow><mo fence=\"false\" maxsize=\"3em\" minsize=\"3em\" symmetric=\"true\">‖</mo></mrow><mrow><mo>…</mo></mrow><mrow><mo fence=\"false\" maxsize=\"3em\" minsize=\"3em\" stretchy=\"true\" symmetric=\"true\">|</mo></mrow><mrow><mo fence=\"false\" maxsize=\"2.4em\" minsize=\"2.4em\" stretchy=\"true\" symmetric=\"true\">|</mo></mrow><mrow><mo fence=\"false\" maxsize=\"1.8em\" minsize=\"1.8em\" stretchy=\"true\" symmetric=\"true\">|</mo></mrow><mrow><mo fence=\"false\" maxsize=\"1.2em\" minsize=\"1.2em\" stretchy=\"true\" symmetric=\"true\">|</mo></mrow><mrow><mi>|</mi></mrow></math>",
        ),
        (
            r"\lfloor \bigl\lfloor \Bigl\lfloor \biggl\lfloor \Biggl\lfloor \dots \Biggr\rceil \biggr\rceil \Bigr\rceil \bigr\rceil \rceil",
            "<math><mrow><mo form=\"prefix\" stretchy=\"false\">⌊</mo><mo fence=\"true\" maxsize=\"1.2em\" minsize=\"1.2em\" symmetric=\"true\">⌊</mo><mo fence=\"true\" maxsize=\"1.8em\" minsize=\"1.8em\" symmetric=\"true\">⌊</mo><mo fence=\"true\" maxsize=\"2.4em\" minsize=\"2.4em\" symmetric=\"true\">⌊</mo><mo fence=\"true\" maxsize=\"3em\" minsize=\"3em\" symmetric=\"true\">⌊</mo><mo>…</mo><mspace width=\"0.1667em\"></mspace><mo fence=\"true\" maxsize=\"3em\" minsize=\"3em\" symmetric=\"true\">⌉</mo><mo fence=\"true\" maxsize=\"2.4em\" minsize=\"2.4em\" symmetric=\"true\">⌉</mo><mo fence=\"true\" maxsize=\"1.8em\" minsize=\"1.8em\" symmetric=\"true\">⌉</mo><mo fence=\"true\" maxsize=\"1.2em\" minsize=\"1.2em\" symmetric=\"true\">⌉</mo><mo form=\"postfix\" stretchy=\"false\">⌉</mo></mrow></math>",
        ),
        (
            r"\uparrow \big\uparrow \Big\uparrow \bigg\uparrow \Bigg\uparrow \dots \Bigg\Downarrow \bigg\Downarrow \Big\Downarrow \big\Downarrow \Downarrow",
            "<math><mrow><mo stretchy=\"false\">↑</mo></mrow><mrow><mo fence=\"false\" maxsize=\"1.2em\" minsize=\"1.2em\" stretchy=\"true\" symmetric=\"true\">↑</mo></mrow><mrow><mo fence=\"false\" maxsize=\"1.8em\" minsize=\"1.8em\" stretchy=\"true\" symmetric=\"true\">↑</mo></mrow><mrow><mo fence=\"false\" maxsize=\"2.4em\" minsize=\"2.4em\" stretchy=\"true\" symmetric=\"true\">↑</mo></mrow><mrow><mo fence=\"false\" maxsize=\"3em\" minsize=\"3em\" stretchy=\"true\" symmetric=\"true\">↑</mo></mrow><mrow><mo>…</mo></mrow><mrow><mo fence=\"false\" maxsize=\"3em\" minsize=\"3em\" stretchy=\"true\" symmetric=\"true\">⇓</mo></mrow><mrow><mo fence=\"false\" maxsize=\"2.4em\" minsize=\"2.4em\" stretchy=\"true\" symmetric=\"true\">⇓</mo></mrow><mrow><mo fence=\"false\" maxsize=\"1.8em\" minsize=\"1.8em\" stretchy=\"true\" symmetric=\"true\">⇓</mo></mrow><mrow><mo fence=\"false\" maxsize=\"1.2em\" minsize=\"1.2em\" stretchy=\"true\" symmetric=\"true\">⇓</mo></mrow><mrow><mo stretchy=\"false\">⇓</mo></mrow></math>",
        ),
        (
            r"\updownarrow\big\updownarrow\Big\updownarrow \bigg\updownarrow \Bigg\updownarrow \dots \Bigg\Updownarrow \bigg\Updownarrow \Big \Updownarrow \big\Updownarrow \Updownarrow",
            "<math><mrow><mo stretchy=\"false\">↕</mo></mrow><mrow><mo fence=\"false\" maxsize=\"1.2em\" minsize=\"1.2em\" stretchy=\"true\" symmetric=\"true\">↕</mo></mrow><mrow><mo fence=\"false\" maxsize=\"1.8em\" minsize=\"1.8em\" stretchy=\"true\" symmetric=\"true\">↕</mo></mrow><mrow><mo fence=\"false\" maxsize=\"2.4em\" minsize=\"2.4em\" stretchy=\"true\" symmetric=\"true\">↕</mo></mrow><mrow><mo fence=\"false\" maxsize=\"3em\" minsize=\"3em\" stretchy=\"true\" symmetric=\"true\">↕</mo></mrow><mrow><mo>…</mo></mrow><mrow><mo fence=\"false\" maxsize=\"3em\" minsize=\"3em\" stretchy=\"true\" symmetric=\"true\">⇕</mo></mrow><mrow><mo fence=\"false\" maxsize=\"2.4em\" minsize=\"2.4em\" stretchy=\"true\" symmetric=\"true\">⇕</mo></mrow><mrow><mo fence=\"false\" maxsize=\"1.8em\" minsize=\"1.8em\" stretchy=\"true\" symmetric=\"true\">⇕</mo></mrow><mrow><mo fence=\"false\" maxsize=\"1.2em\" minsize=\"1.2em\" stretchy=\"true\" symmetric=\"true\">⇕</mo></mrow><mrow><mo stretchy=\"false\">⇕</mo></mrow></math>",
        ),
        (
            r"/ \big/ \Big/ \bigg/ \Bigg/ \dots \Bigg\backslash \bigg\backslash \Big \backslash \big\backslash \backslash",
            "<math><mrow><mi>/</mi><mo fence=\"false\" maxsize=\"1.2em\" minsize=\"1.2em\" symmetric=\"true\">∕</mo></mrow><mrow><mo fence=\"false\" maxsize=\"1.8em\" minsize=\"1.8em\" symmetric=\"true\">∕</mo></mrow><mrow><mo fence=\"false\" maxsize=\"2.4em\" minsize=\"2.4em\" symmetric=\"true\">∕</mo></mrow><mrow><mo fence=\"false\" maxsize=\"3em\" minsize=\"3em\" symmetric=\"true\">∕</mo></mrow><mrow><mo>…</mo></mrow><mrow><mo fence=\"false\" maxsize=\"3em\" minsize=\"3em\" stretchy=\"true\" symmetric=\"true\">∖</mo></mrow><mrow><mo fence=\"false\" maxsize=\"2.4em\" minsize=\"2.4em\" stretchy=\"true\" symmetric=\"true\">∖</mo></mrow><mrow><mo fence=\"false\" maxsize=\"1.8em\" minsize=\"1.8em\" stretchy=\"true\" symmetric=\"true\">∖</mo></mrow><mrow><mo fence=\"false\" maxsize=\"1.2em\" minsize=\"1.2em\" stretchy=\"true\" symmetric=\"true\">∖</mo></mrow><mrow><mi>\\</mi></mrow></math>",
        ),
        (
            r"\Alpha \Beta \Gamma \Delta \Epsilon \Zeta \Eta \Theta",
            "<math><mrow><mrow><mi mathvariant=\"normal\">Α</mi></mrow><mrow><mi mathvariant=\"normal\">Β</mi></mrow><mrow><mi mathvariant=\"normal\">Γ</mi></mrow><mrow><mi mathvariant=\"normal\">Δ</mi></mrow><mrow><mi mathvariant=\"normal\">Ε</mi></mrow><mrow><mi mathvariant=\"normal\">Ζ</mi></mrow><mrow><mi mathvariant=\"normal\">Η</mi></mrow><mrow><mi mathvariant=\"normal\">Θ</mi></mrow></mrow></math>",
        ),
        (
            r"\Iota \Kappa \Lambda \Mu \Nu \Xi \Omicron \Pi",
            "<math><mrow><mrow><mi mathvariant=\"normal\">Ι</mi></mrow><mrow><mi mathvariant=\"normal\">Κ</mi></mrow><mrow><mi mathvariant=\"normal\">Λ</mi></mrow><mrow><mi mathvariant=\"normal\">Μ</mi></mrow><mrow><mi mathvariant=\"normal\">Ν</mi></mrow><mrow><mi mathvariant=\"normal\">Ξ</mi></mrow><mrow><mi mathvariant=\"normal\">Ο</mi></mrow><mrow><mi mathvariant=\"normal\">Π</mi></mrow></mrow></math>",
        ),
        (
            r"\Rho \Sigma \Tau \Upsilon \Phi \Chi \Psi \Omega",
            "<math><mrow><mrow><mi mathvariant=\"normal\">Ρ</mi></mrow><mrow><mi mathvariant=\"normal\">Σ</mi></mrow><mrow><mi mathvariant=\"normal\">Τ</mi></mrow><mrow><mi mathvariant=\"normal\">Υ</mi></mrow><mrow><mi mathvariant=\"normal\">Φ</mi></mrow><mrow><mi mathvariant=\"normal\">Χ</mi></mrow><mrow><mi mathvariant=\"normal\">Ψ</mi></mrow><mrow><mi mathvariant=\"normal\">Ω</mi></mrow></mrow></math>",
        ),
        (
            r"\alpha \beta \gamma \delta \epsilon \zeta \eta \theta",
            "<math><mrow><mi>α</mi><mi>β</mi><mi>γ</mi><mi>δ</mi><mi>ϵ</mi><mi>ζ</mi><mi>η</mi><mi>θ</mi></mrow></math>",
        ),
        (
            r"\iota \kappa \lambda \mu \nu \xi \omicron \pi",
            "<math><mrow><mi>ι</mi><mi>κ</mi><mi>λ</mi><mi>μ</mi><mi>ν</mi><mi>ξ</mi><mi>ο</mi><mi>π</mi></mrow></math>",
        ),
        (
            r"\rho \sigma \tau \upsilon \phi \chi \psi \omega",
            "<math><mrow><mi>ρ</mi><mi>σ</mi><mi>τ</mi><mi>υ</mi><mi>ϕ</mi><mi>χ</mi><mi>ψ</mi><mi>ω</mi></mrow></math>",
        ),
        (
            r"\varGamma \varDelta \varTheta \varLambda \varXi \varPi \varSigma \varPhi \varUpsilon \varOmega",
            "<math><mrow><mi>𝛤</mi><mi>𝛥</mi><mi>𝛩</mi><mi>𝛬</mi><mi>𝛯</mi><mi>𝛱</mi><mi>𝛴</mi><mi>𝛷</mi><mi>𝛶</mi><mi>𝛺</mi></mrow></math>",
        ),
        (
            r"\varepsilon \digamma \varkappa \varpi \varrho \varsigma \vartheta \varphi",
            "<math><mrow><mi>ε</mi><mi>ϝ</mi><mi>ϰ</mi><mi>ϖ</mi><mi>ϱ</mi><mi>ς</mi><mi>ϑ</mi><mi>φ</mi></mrow></math>",
        ),
        (
            r"\aleph \beth \gimel \daleth",
            "<math><mrow><mi>ℵ</mi><mi>ℶ</mi><mi>ℷ</mi><mi>ℸ</mi></mrow></math>",
        ),
        (
            r"\mathbb{ABCDEFGHI} \\ \mathbb{JKLMNOPQR} \\ \mathbb{STUVWXYZ}",
            "<math><mtable columnalign=\"left\" rowspacing=\"0em\"><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mi>𝔸</mi><mi>𝔹</mi><mi>ℂ</mi><mi>𝔻</mi><mi>𝔼</mi><mi>𝔽</mi><mi>𝔾</mi><mi>ℍ</mi><mi>𝕀</mi></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mi>𝕁</mi><mi>𝕂</mi><mi>𝕃</mi><mi>𝕄</mi><mi>ℕ</mi><mi>𝕆</mi><mi>ℙ</mi><mi>ℚ</mi><mi>ℝ</mi></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mi>𝕊</mi><mi>𝕋</mi><mi>𝕌</mi><mi>𝕍</mi><mi>𝕎</mi><mi>𝕏</mi><mi>𝕐</mi><mi>ℤ</mi></mrow></mrow></mtd></mtr></mtable></math>",
        ),
        (
            r"\mathbf{ABCDEFGHI} \\ \mathbf{JKLMNOPQR} \\ \mathbf{STUVWXYZ} \\ \mathbf{abcdefghijklm} \\ \mathbf{nopqrstuvwxyz} \\ \mathbf{0123456789}",
            "<math><mtable columnalign=\"left\" rowspacing=\"0em\"><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mi>𝐀</mi><mi>𝐁</mi><mi>𝐂</mi><mi>𝐃</mi><mi>𝐄</mi><mi>𝐅</mi><mi>𝐆</mi><mi>𝐇</mi><mi>𝐈</mi></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mi>𝐉</mi><mi>𝐊</mi><mi>𝐋</mi><mi>𝐌</mi><mi>𝐍</mi><mi>𝐎</mi><mi>𝐏</mi><mi>𝐐</mi><mi>𝐑</mi></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mi>𝐒</mi><mi>𝐓</mi><mi>𝐔</mi><mi>𝐕</mi><mi>𝐖</mi><mi>𝐗</mi><mi>𝐘</mi><mi>𝐙</mi></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mi>𝐚</mi><mi>𝐛</mi><mi>𝐜</mi><mi>𝐝</mi><mi>𝐞</mi><mi>𝐟</mi><mi>𝐠</mi><mi>𝐡</mi><mi>𝐢</mi><mi>𝐣</mi><mi>𝐤</mi><mi>𝐥</mi><mi>𝐦</mi></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mi>𝐧</mi><mi>𝐨</mi><mi>𝐩</mi><mi>𝐪</mi><mi>𝐫</mi><mi>𝐬</mi><mi>𝐭</mi><mi>𝐮</mi><mi>𝐯</mi><mi>𝐰</mi><mi>𝐱</mi><mi>𝐲</mi><mi>𝐳</mi></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mn>𝟎𝟏𝟐𝟑𝟒𝟓𝟔𝟕𝟖𝟗</mn></mrow></mtd></mtr></mtable></math>",
        ),
        (
            r"\boldsymbol{\Alpha \Beta \Gamma \Delta \Epsilon \Zeta \Eta \Theta}",
            "<math><mrow style=\"font-weight:bold;\"><mi>𝚨</mi><mi>𝚩</mi><mi>𝚪</mi><mi>𝚫</mi><mi>𝚬</mi><mi>𝚭</mi><mi>𝚮</mi><mi>𝚯</mi></mrow></math>",
        ),
        (
            r"\boldsymbol{\Iota \Kappa \Lambda \Mu \Nu \Xi \Omicron \Pi}",
            "<math><mrow style=\"font-weight:bold;\"><mi>𝚰</mi><mi>𝚱</mi><mi>𝚲</mi><mi>𝚳</mi><mi>𝚴</mi><mi>𝚵</mi><mi>𝚶</mi><mi>𝚷</mi></mrow></math>",
        ),
        (
            r"\boldsymbol{\Rho \Sigma \Tau \Upsilon \Phi \Chi \Psi \Omega}",
            "<math><mrow style=\"font-weight:bold;\"><mi>𝚸</mi><mi>𝚺</mi><mi>𝚻</mi><mi>𝚼</mi><mi>𝚽</mi><mi>𝚾</mi><mi>𝚿</mi><mi>𝛀</mi></mrow></math>",
        ),
        (
            r"\boldsymbol{\alpha \beta \gamma \delta \epsilon \zeta \eta \theta}",
            "<math><mrow style=\"font-weight:bold;\"><mi>𝜶</mi><mi>𝜷</mi><mi>𝜸</mi><mi>𝜹</mi><mi>𝝐</mi><mi>𝜻</mi><mi>𝜼</mi><mi>𝜽</mi></mrow></math>",
        ),
        (
            r"\boldsymbol{\iota \kappa \lambda \mu \nu \xi \omicron \pi}",
            "<math><mrow style=\"font-weight:bold;\"><mi>𝜾</mi><mi>𝜿</mi><mi>𝝀</mi><mi>𝝁</mi><mi>𝝂</mi><mi>𝝃</mi><mi>𝝄</mi><mi>𝝅</mi></mrow></math>",
        ),
        (
            r"\boldsymbol{\rho \sigma \tau \upsilon \phi \chi \psi \omega}",
            "<math><mrow style=\"font-weight:bold;\"><mi>𝝆</mi><mi>𝝈</mi><mi>𝝉</mi><mi>𝝊</mi><mi>𝝓</mi><mi>𝝌</mi><mi>𝝍</mi><mi>𝝎</mi></mrow></math>",
        ),
        (
            r"\boldsymbol{\varepsilon\digamma\varkappa \varpi}",
            "<math><mrow style=\"font-weight:bold;\"><mi>𝜺</mi><mi>ϝ</mi><mi>𝝒</mi><mi>𝝕</mi></mrow></math>",
        ),
        (
            r"\boldsymbol{\varrho\varsigma\vartheta\varphi}",
            "<math><mrow style=\"font-weight:bold;\"><mi>𝝔</mi><mi>𝝇</mi><mi>𝝑</mi><mi>𝝋</mi></mrow></math>",
        ),
        (
            r"\mathit{0123456789}",
            "<math><mstyle style=\"font-style:italic;font-family:Cambria, 'Times New Roman', serif;\"><mn>0123456789</mn></mstyle></math>",
        ),
        (
            r"\mathit{\Alpha \Beta \Gamma \Delta \Epsilon \Zeta \Eta \Theta}",
            "<math><mrow><mi>Α</mi><mi>Β</mi><mi>Γ</mi><mi>Δ</mi><mi>Ε</mi><mi>Ζ</mi><mi>Η</mi><mi>Θ</mi></mrow></math>",
        ),
        (
            r"\mathit{\Iota \Kappa \Lambda \Mu \Nu \Xi \Omicron \Pi}",
            "<math><mrow><mi>Ι</mi><mi>Κ</mi><mi>Λ</mi><mi>Μ</mi><mi>Ν</mi><mi>Ξ</mi><mi>Ο</mi><mi>Π</mi></mrow></math>",
        ),
        (
            r"\mathit{\Rho \Sigma \Tau \Upsilon \Phi \Chi \Psi \Omega}",
            "<math><mrow><mi>Ρ</mi><mi>Σ</mi><mi>Τ</mi><mi>Υ</mi><mi>Φ</mi><mi>Χ</mi><mi>Ψ</mi><mi>Ω</mi></mrow></math>",
        ),
        (
            r"\boldsymbol{\varGamma \varDelta \varTheta \varLambda}",
            "<math><mrow style=\"font-weight:bold;\"><mi>𝜞</mi><mi>𝜟</mi><mi>𝜣</mi><mi>𝜦</mi></mrow></math>",
        ),
        (
            r"\boldsymbol{\varXi \varPi \varSigma \varUpsilon \varOmega}",
            "<math><mrow style=\"font-weight:bold;\"><mi>𝜩</mi><mi>𝜫</mi><mi>𝜮</mi><mi>𝜰</mi><mi>𝜴</mi></mrow></math>",
        ),
        (
            r"\mathrm{ABCDEFGHI} \\ \mathrm{JKLMNOPQR} \\ \mathrm{STUVWXYZ} \\ \mathrm{abcdefghijklm} \\ \mathrm{nopqrstuvwxyz} \\ \mathrm{0123456789}",
            "<math><mtable columnalign=\"left\" rowspacing=\"0em\"><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mtext></mtext><mi>ABCDEFGHI</mi></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mtext></mtext><mi>JKLMNOPQR</mi></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mtext></mtext><mi>STUVWXYZ</mi></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mtext></mtext><mi>abcdefghijklm</mi></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mtext></mtext><mi>nopqrstuvwxyz</mi></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mn>0123456789</mn></mrow></mtd></mtr></mtable></math>",
        ),
        (
            r"\mathsf{ABCDEFGHI} \\ \mathsf{JKLMNOPQR} \\ \mathsf{STUVWXYZ} \\ \mathsf{abcdefghijklm} \\ \mathsf{nopqrstuvwxyz} \\ \mathsf{0123456789}",
            "<math><mtable columnalign=\"left\" rowspacing=\"0em\"><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mi>𝖠</mi><mi>𝖡</mi><mi>𝖢</mi><mi>𝖣</mi><mi>𝖤</mi><mi>𝖥</mi><mi>𝖦</mi><mi>𝖧</mi><mi>𝖨</mi></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mi>𝖩</mi><mi>𝖪</mi><mi>𝖫</mi><mi>𝖬</mi><mi>𝖭</mi><mi>𝖮</mi><mi>𝖯</mi><mi>𝖰</mi><mi>𝖱</mi></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mi>𝖲</mi><mi>𝖳</mi><mi>𝖴</mi><mi>𝖵</mi><mi>𝖶</mi><mi>𝖷</mi><mi>𝖸</mi><mi>𝖹</mi></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mi>𝖺</mi><mi>𝖻</mi><mi>𝖼</mi><mi>𝖽</mi><mi>𝖾</mi><mi>𝖿</mi><mi>𝗀</mi><mi>𝗁</mi><mi>𝗂</mi><mi>𝗃</mi><mi>𝗄</mi><mi>𝗅</mi><mi>𝗆</mi></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mi>𝗇</mi><mi>𝗈</mi><mi>𝗉</mi><mi>𝗊</mi><mi>𝗋</mi><mi>𝗌</mi><mi>𝗍</mi><mi>𝗎</mi><mi>𝗏</mi><mi>𝗐</mi><mi>𝗑</mi><mi>𝗒</mi><mi>𝗓</mi></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mn>𝟢𝟣𝟤𝟥𝟦𝟧𝟨𝟩𝟪𝟫</mn></mrow></mtd></mtr></mtable></math>",
        ),
        (
            r"\mathsf{\Alpha \Beta \Gamma \Delta \Epsilon \Zeta \Eta \Theta}",
            "<math><mrow><mi>𝝖</mi><mi>𝝗</mi><mi>𝝘</mi><mi>𝝙</mi><mi>𝝚</mi><mi>𝝛</mi><mi>𝝜</mi><mi>𝝝</mi></mrow></math>",
        ),
        (
            r"\mathsf{\Iota \Kappa \Lambda \Mu \Nu \Xi \Omicron \Pi}",
            "<math><mrow><mi>𝝞</mi><mi>𝝟</mi><mi>𝝠</mi><mi>𝝡</mi><mi>𝝢</mi><mi>𝝣</mi><mi>𝝤</mi><mi>𝝥</mi></mrow></math>",
        ),
        (
            r"\mathsf{\Rho \Sigma \Tau \Upsilon \Phi \Chi \Psi \Omega}",
            "<math><mrow><mi>𝝦</mi><mi>𝝨</mi><mi>𝝩</mi><mi>𝝪</mi><mi>𝝫</mi><mi>𝝬</mi><mi>𝝭</mi><mi>𝝮</mi></mrow></math>",
        ),
        (
            r"\mathcal{ABCDEFGHI} \\ \mathcal{JKLMNOPQR} \\ \mathcal{STUVWXYZ} \\ \mathcal{abcdefghi} \\ \mathcal{jklmnopqr} \\ \mathcal{stuvwxyz}",
            "<math><mtable columnalign=\"left\" rowspacing=\"0em\"><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mi>𝒜</mi><mi>ℬ</mi><mi>𝒞</mi><mi>𝒟</mi><mi>ℰ</mi><mi>ℱ</mi><mi>𝒢</mi><mi>ℋ</mi><mi>ℐ</mi></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mi>𝒥</mi><mi>𝒦</mi><mi>ℒ</mi><mi>ℳ</mi><mi>𝒩</mi><mi>𝒪</mi><mi>𝒫</mi><mi>𝒬</mi><mi>ℛ</mi></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mi>𝒮</mi><mi>𝒯</mi><mi>𝒰</mi><mi>𝒱</mi><mi>𝒲</mi><mi>𝒳</mi><mi>𝒴</mi><mi>𝒵</mi></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mi>𝒶</mi><mi>𝒷</mi><mi>𝒸</mi><mi>𝒹</mi><mi>ℯ</mi><mi>𝒻</mi><mi>ℊ</mi><mi>𝒽</mi><mi>𝒾</mi></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mi>𝒿</mi><mi>𝓀</mi><mi>𝓁</mi><mi>𝓂</mi><mi>𝓃</mi><mi>ℴ</mi><mi>𝓅</mi><mi>𝓆</mi><mi>𝓇</mi></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mi>𝓈</mi><mi>𝓉</mi><mi>𝓊</mi><mi>𝓋</mi><mi>𝓌</mi><mi>𝓍</mi><mi>𝓎</mi><mi>𝓏</mi></mrow></mrow></mtd></mtr></mtable></math>",
        ),
        (
            r"\mathfrak{ABCDEFGHI} \\ \mathfrak{JKLMNOPQR} \\ \mathfrak{STUVWXYZ} \\ \mathfrak{abcdefghi} \\ \mathfrak{jklmnopqr} \\ \mathfrak{stuvwxyz}",
            "<math><mtable columnalign=\"left\" rowspacing=\"0em\"><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mi>𝔄</mi><mi>𝔅</mi><mi>ℭ</mi><mi>𝔇</mi><mi>𝔈</mi><mi>𝔉</mi><mi>𝔊</mi><mi>ℌ</mi><mi>ℑ</mi></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mi>𝔍</mi><mi>𝔎</mi><mi>𝔏</mi><mi>𝔐</mi><mi>𝔑</mi><mi>𝔒</mi><mi>𝔓</mi><mi>𝔔</mi><mi>ℜ</mi></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mi>𝔖</mi><mi>𝔗</mi><mi>𝔘</mi><mi>𝔙</mi><mi>𝔚</mi><mi>𝔛</mi><mi>𝔜</mi><mi>ℨ</mi></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mi>𝔞</mi><mi>𝔟</mi><mi>𝔠</mi><mi>𝔡</mi><mi>𝔢</mi><mi>𝔣</mi><mi>𝔤</mi><mi>𝔥</mi><mi>𝔦</mi></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mi>𝔧</mi><mi>𝔨</mi><mi>𝔩</mi><mi>𝔪</mi><mi>𝔫</mi><mi>𝔬</mi><mi>𝔭</mi><mi>𝔮</mi><mi>𝔯</mi></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mi>𝔰</mi><mi>𝔱</mi><mi>𝔲</mi><mi>𝔳</mi><mi>𝔴</mi><mi>𝔵</mi><mi>𝔶</mi><mi>𝔷</mi></mrow></mrow></mtd></mtr></mtable></math>",
        ),
        (
            r"{\scriptstyle\text{abcdefghijklm}}",
            "<math><mstyle displaystyle=\"false\" scriptlevel=\"1\"><mtext>abcdefghijklm</mtext></mstyle></math>",
        ),
        (
            r"x y z",
            "<math><mrow><mi>x</mi><mi>y</mi><mi>z</mi></mrow></math>",
        ),
        (r"\text{x y z}", "<math><mtext>x y z</mtext></math>"),
        (
            r"\text{if} n \text{is even}",
            "<math><mrow><mtext>if</mtext><mi>n</mi><mtext>is even</mtext></mrow></math>",
        ),
        (
            r"\text{if }n\text{ is even}",
            "<math><mrow><mtext>if </mtext><mi>n</mi><mtext> is even</mtext></mrow></math>",
        ),
        (
            r"\text{if}~n\ \text{is even}",
            "<math><mrow><mtext>if</mtext><mtext> </mtext><mi>n</mi><mtext> </mtext><mtext>is even</mtext></mrow></math>",
        ),
        (
            r"{\color{Blue}x^2}+{\color{Orange}2x}- {\color{LimeGreen}1}",
            "<math><mrow><mrow><msup style=\"color:#303494;\"><mi>x</mi><mn>2</mn></msup></mrow><mo>+</mo></mrow><mrow><mrow><mn style=\"color:#f8843c;\">2</mn><mi style=\"color:#f8843c;\">x</mi></mrow><mo>−</mo></mrow><mrow><mrow><mn style=\"color:#90c43c;\">1</mn></mrow></mrow></math>",
        ),
        (
            r"x_{1,2}=\frac{{\color{Blue}-b}\pm \sqrt{\color{Red}b^2-4ac}}{\color{Green}2a }",
            "<math><mrow><msub><mi>x</mi><mn>1,2</mn></msub><mo>=</mo></mrow><mrow><mfrac><mrow><mrow><mo form=\"prefix\" stretchy=\"false\" style=\"color:#303494;\">−</mo><mi style=\"color:#303494;\">b</mi></mrow><mo>±</mo><msqrt><mrow><msup style=\"color:#f01c24;\"><mi>b</mi><mn>2</mn></msup><mo style=\"color:#f01c24;\">−</mo><mn style=\"color:#f01c24;\">4</mn><mi style=\"color:#f01c24;\">a</mi><mi style=\"color:#f01c24;\">c</mi></mrow></msqrt></mrow><mrow><mn style=\"color:#08a44c;\">2</mn><mi style=\"color:#08a44c;\">a</mi></mrow></mfrac></mrow></math>",
        ),
        (
            r"{\color{Blue}x^2}+{\color{Orange}2x}- {\color{LimeGreen}1}",
            "<math><mrow><mrow><msup style=\"color:#303494;\"><mi>x</mi><mn>2</mn></msup></mrow><mo>+</mo></mrow><mrow><mrow><mn style=\"color:#f8843c;\">2</mn><mi style=\"color:#f8843c;\">x</mi></mrow><mo>−</mo></mrow><mrow><mrow><mn style=\"color:#90c43c;\">1</mn></mrow></mrow></math>",
        ),
        (
            r"\color{Blue}x^2\color{Black}+\color{Orange} 2x\color{Black}-\color{LimeGreen}1",
            "<math><mrow><msup style=\"color:#303494;\"><mi>x</mi><mn>2</mn></msup><mo style=\"color:Black;\">+</mo></mrow><mrow><mn style=\"color:#f8843c;\">2</mn><mi style=\"color:#f8843c;\">x</mi><mo style=\"color:Black;\">−</mo></mrow><mrow><mn style=\"color:#90c43c;\">1</mn></mrow></math>",
        ),
        (
            r"\color{Blue}{x^2}+\color{Orange}{2x}- \color{LimeGreen}{1}",
            "<math><mrow><msup style=\"color:#303494;\"><mi>x</mi><mn>2</mn></msup><mo style=\"color:#303494;\">+</mo></mrow><mrow><mrow style=\"color:#f8843c;\"><mn>2</mn><mi>x</mi></mrow><mo style=\"color:#f8843c;\">−</mo></mrow><mrow><mn style=\"color:#90c43c;\">1</mn></mrow></math>",
        ),
        (
            r"\definecolor{myorange}{rgb}{1,0.65,0.4} \color{myorange}e^{i \pi}\color{Black} + 1= 0",
            "<math><mrow><msup style=\"color:#ffa666;\"><mi>e</mi><mrow><mi>i</mi><mi>π</mi></mrow></msup><mo style=\"color:Black;\">+</mo></mrow><mrow><mn style=\"color:Black;\">1</mn><mo style=\"color:Black;\">=</mo></mrow><mrow><mn style=\"color:Black;\">0</mn></mrow></math>",
        ),
        (
            r"a \qquad b \\ a \quad b \\ a\ b \\ a \text{ } b \\ a\;b \\ a\,b \\ ab \\ a b \\ \mathit{ab} \\ a\!b",
            "<math><mtable columnalign=\"left\" rowspacing=\"0em\"><mtr><mtd style=\"text-align:left;\"><mrow><mi>a</mi><mspace width=\"2em\"></mspace><mi>b</mi></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mi>a</mi><mspace width=\"1em\"></mspace><mi>b</mi></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mi>a</mi><mtext> </mtext><mi>b</mi></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mi>a</mi><mtext> </mtext><mi>b</mi></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mi>a</mi><mspace width=\"0.2778em\"></mspace><mi>b</mi></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mi>a</mi><mspace width=\"0.1667em\"></mspace><mi>b</mi></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mi>a</mi><mi>b</mi></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mi>a</mi><mi>b</mi></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mrow><mi>a</mi><mi>b</mi></mrow></mrow><mo linebreak=\"newline\"></mo></mtd></mtr><mtr><mtd style=\"text-align:left;\"><mrow><mi>a</mi><mspace style=\"margin-left:-0.1667em;\" width=\"-0.1667em\"></mspace><mi>b</mi></mrow></mtd></mtr></mtable></math>",
        ),
        (
            r"| \uparrow \rangle",
            "<math><mrow><mi>|</mi><mo stretchy=\"false\">↑</mo></mrow><mrow><mo form=\"postfix\" stretchy=\"false\">⟩</mo></mrow></math>",
        ),
        (
            r"\left| \uparrow \right\rangle",
            "<math><mrow><mo fence=\"true\" form=\"prefix\">|</mo><mo lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">↑</mo><mo fence=\"true\" form=\"postfix\">⟩</mo></mrow></math>",
        ),
        (
            r"| {\uparrow} \rangle",
            "<math><mrow><mi>|</mi><mo lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">↑</mo></mrow><mrow><mo form=\"postfix\" stretchy=\"false\">⟩</mo></mrow></math>",
        ),
        (
            r"| \mathord\uparrow \rangle",
            "<math><mrow><mi>|</mi><mi>↑</mi><mo form=\"postfix\" stretchy=\"false\">⟩</mo></mrow></math>",
        ),
        (
            r"\wideparen{AB}",
            "<math><mover><mrow><mi>A</mi><mi>B</mi></mrow><mo stretchy=\"true\" style=\"math-style:normal;math-depth:0;\">⏜</mo></mover></math>",
        ),
        (
            r"\dddot{x}",
            "<math><mover><mi>x</mi><mo stretchy=\"false\" style=\"math-style:normal;math-depth:0;\">…</mo></mover></math>",
        ),
        (
            r"\sout{q}",
            "<math><mrow style=\"background-image:linear-gradient(black, black);background-repeat:no-repeat;background-size:100% 1.5px;background-position:0 center;\"><mi>q</mi></mrow></math>",
        ),
        (
            r"\mathrlap{\,/}{=}",
            "<math><mrow><mpadded width=\"0px\"><mrow><mspace width=\"0.1667em\"></mspace><mi>/</mi></mrow></mpadded><mo lspace=\"0em\" rspace=\"0em\">=</mo></mrow></math>",
        ),
        (
            r"\text{\textsf{textual description}}",
            "<math><mtext>𝗍𝖾𝗑𝗍𝗎𝖺𝗅 𝖽𝖾𝗌𝖼𝗋𝗂𝗉𝗍𝗂𝗈𝗇</mtext></math>",
        ),
        (r"α π", "<math><mrow><mi>α</mi><mi>π</mi></mrow></math>"),
        (
            r"ax^2 + bx + c = 0",
            "<math><mrow><mi>a</mi><msup><mi>x</mi><mn>2</mn></msup><mo>+</mo></mrow><mrow><mi>b</mi><mi>x</mi><mo>+</mo></mrow><mrow><mi>c</mi><mo>=</mo></mrow><mrow><mn>0</mn></mrow></math>",
        ),
        (
            r"x=\frac{-b\pm\sqrt{b^2-4ac}}{2a}",
            "<math><mrow><mi>x</mi><mo>=</mo></mrow><mrow><mfrac><mrow><mo form=\"prefix\" lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">−</mo><mi>b</mi><mo>±</mo><msqrt><mrow><msup><mi>b</mi><mn>2</mn></msup><mo>−</mo><mn>4</mn><mi>a</mi><mi>c</mi></mrow></msqrt></mrow><mrow><mn>2</mn><mi>a</mi></mrow></mfrac></mrow></math>",
        ),
        (
            r"\left( \frac{\left(3-x\right) \times 2}{3-x} \right)",
            "<math><mrow><mo fence=\"true\" form=\"prefix\">(</mo><mfrac><mrow><mrow><mo fence=\"true\" form=\"prefix\">(</mo><mn>3</mn><mo>−</mo><mi>x</mi><mo fence=\"true\" form=\"postfix\">)</mo></mrow><mo>×</mo><mn>2</mn></mrow><mrow><mn>3</mn><mo>−</mo><mi>x</mi></mrow></mfrac><mo fence=\"true\" form=\"postfix\">)</mo></mrow></math>",
        ),
        (
            r"\det(\mathsf{A}-\lambda\mathsf{I}) = 0",
            "<math><mrow><mrow><mi>det</mi><mo>⁡</mo></mrow><mo form=\"prefix\" stretchy=\"false\">(</mo><mi>𝖠</mi><mo>−</mo><mi>λ</mi><mi>𝖨</mi><mo form=\"postfix\" stretchy=\"false\">)</mo><mo>=</mo></mrow><mrow><mn>0</mn></mrow></math>",
        ),
        (
            r"u'' + p(x)u' + q(x)u=f(x),\quad x>a",
            "<math><mrow><msup><mi>u</mi><mrow><mo lspace=\"0em\" rspace=\"0em\">′</mo><mo lspace=\"0em\" rspace=\"0em\">′</mo></mrow></msup><mo>+</mo></mrow><mrow><mi>p</mi><mo form=\"prefix\" stretchy=\"false\">(</mo><mi>x</mi><mo form=\"postfix\" stretchy=\"false\">)</mo><msup><mi>u</mi><mo lspace=\"0em\" rspace=\"0em\">′</mo></msup><mo>+</mo></mrow><mrow><mi>q</mi><mo form=\"prefix\" stretchy=\"false\">(</mo><mi>x</mi><mo form=\"postfix\" stretchy=\"false\">)</mo><mi>u</mi><mo>=</mo></mrow><mrow><mi>f</mi><mo form=\"prefix\" stretchy=\"false\">(</mo><mi>x</mi><mo form=\"postfix\" stretchy=\"false\">)</mo><mo>,</mo><mspace width=\"1em\"></mspace></mrow><mrow><mi>x</mi><mo>&gt;</mo></mrow><mrow><mi>a</mi></mrow></math>",
        ),
        (
            r"|\bar{z}| = |z|, |(\bar{z})^n| = |z|^n, \arg(z^n) = n \arg(z)",
            "<math><mrow><mi>|</mi><mover><mi>z</mi><mo stretchy=\"false\" style=\"math-style:normal;math-depth:0;\">‾</mo></mover><mi>|</mi><mo>=</mo></mrow><mrow><mi>|</mi><mi>z</mi><mi>|</mi><mo>,</mo></mrow><mrow><mi>|</mi><mo form=\"prefix\" stretchy=\"false\">(</mo><mover><mi>z</mi><mo stretchy=\"false\" style=\"math-style:normal;math-depth:0;\">‾</mo></mover><msup><mo form=\"postfix\" stretchy=\"false\">)</mo><mi>n</mi></msup><mi>|</mi><mo>=</mo><mi>|</mi><mi>z</mi><msup><mi>|</mi><mi>n</mi></msup><mo>,</mo><mrow><mi>arg</mi><mo>⁡</mo></mrow><mo form=\"prefix\" stretchy=\"false\">(</mo><msup><mi>z</mi><mi>n</mi></msup><mo form=\"postfix\" stretchy=\"false\">)</mo><mo>=</mo><mi>n</mi><mrow><mspace width=\"0.1667em\"></mspace><mi>arg</mi><mo>⁡</mo></mrow><mo form=\"prefix\" stretchy=\"false\">(</mo><mi>z</mi><mo form=\"postfix\" stretchy=\"false\">)</mo></mrow></math>",
        ),
        (
            r"\phi_n(\kappa) = 0.033C_n^2\kappa^{-11/3}, \quad\frac{1}{L_0}\ll\kappa\ll\frac{1}{l_0}",
            "<math><mrow><msub><mi>ϕ</mi><mi>n</mi></msub><mo form=\"prefix\" stretchy=\"false\">(</mo><mi>κ</mi><mo form=\"postfix\" stretchy=\"false\">)</mo><mo>=</mo></mrow><mrow><mn>0.033</mn><msubsup><mi>C</mi><mi>n</mi><mn>2</mn></msubsup><msup><mi>κ</mi><mrow><mo lspace=\"0em\" rspace=\"0em\">−</mo><mn>11</mn><mi>/</mi><mn>3</mn></mrow></msup><mo>,</mo><mspace width=\"1em\"></mspace></mrow><mrow><mfrac><mn>1</mn><msub><mi>L</mi><mn>0</mn></msub></mfrac><mo>≪</mo></mrow><mrow><mi>κ</mi><mo>≪</mo></mrow><mrow><mfrac><mn>1</mn><msub><mi>l</mi><mn>0</mn></msub></mfrac></mrow></math>",
        ),
        (
            r"f(x) = \begin{cases} 1 & -1 \le x < 0 \\ \frac{1}{2} & x = 0 \\ 1 - x^2 & \text{otherwise} \end{cases}",
            "<math><mrow><mi>f</mi><mo form=\"prefix\" stretchy=\"false\">(</mo><mi>x</mi><mo form=\"postfix\" stretchy=\"false\">)</mo><mo>=</mo></mrow><mrow><mrow><mo fence=\"true\" form=\"prefix\">{</mo><mtable><mtr><mtd class=\"tml-left\" style=\"padding:0.5ex 0em 0.5ex 0em;\"><mn>1</mn></mtd><mtd class=\"tml-left\" style=\"padding:0.5ex 0em 0.5ex 1em;\"><mrow><mo>−</mo><mn>1</mn><mo>≤</mo><mi>x</mi><mo>&lt;</mo><mn>0</mn></mrow></mtd></mtr><mtr><mtd class=\"tml-left\" style=\"padding:0.5ex 0em 0.5ex 0em;\"><mfrac><mn>1</mn><mn>2</mn></mfrac></mtd><mtd class=\"tml-left\" style=\"padding:0.5ex 0em 0.5ex 1em;\"><mrow><mi>x</mi><mo>=</mo><mn>0</mn></mrow></mtd></mtr><mtr><mtd class=\"tml-left\" style=\"padding:0.5ex 0em 0.5ex 0em;\"><mrow><mn>1</mn><mo>−</mo><msup><mi>x</mi><mn>2</mn></msup></mrow></mtd><mtd class=\"tml-left\" style=\"padding:0.5ex 0em 0.5ex 1em;\"><mtext>otherwise</mtext></mtd></mtr></mtable><mo fence=\"true\" form=\"postfix\"></mo></mrow></mrow></math>",
        ),
        (
            r"{}_pF_q(a_1,\dots,a_p;c_1,\dots,c_q;z) = \sum_{n=0}^\infty \frac{(a_1)_n\cdots(a_p)_n} {(c_1)_n\cdots(c_q)_n}\frac{z^n}{n!}",
            "<math><mrow><msub><mrow></mrow><mi>p</mi></msub><msub><mi>F</mi><mi>q</mi></msub><mo form=\"prefix\" stretchy=\"false\">(</mo><msub><mi>a</mi><mn>1</mn></msub><mo>,</mo><mo>…</mo><mo>,</mo><msub><mi>a</mi><mi>p</mi></msub><mo separator=\"true\">;</mo><msub><mi>c</mi><mn>1</mn></msub><mo>,</mo><mo>…</mo><mo>,</mo><msub><mi>c</mi><mi>q</mi></msub><mo separator=\"true\">;</mo><mi>z</mi><mo form=\"postfix\" stretchy=\"false\">)</mo><mo>=</mo></mrow><mrow><msubsup><mo movablelimits=\"false\">∑</mo><mrow><mi>n</mi><mo>=</mo><mn>0</mn></mrow><mi>∞</mi></msubsup><mfrac><mrow><mo form=\"prefix\" lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">(</mo><msub><mi>a</mi><mn>1</mn></msub><msub><mo form=\"postfix\" stretchy=\"false\">)</mo><mi>n</mi></msub><mo>⋯</mo><mo form=\"prefix\" stretchy=\"false\">(</mo><msub><mi>a</mi><mi>p</mi></msub><msub><mo form=\"postfix\" stretchy=\"false\">)</mo><mi>n</mi></msub></mrow><mrow><mo form=\"prefix\" lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">(</mo><msub><mi>c</mi><mn>1</mn></msub><msub><mo form=\"postfix\" stretchy=\"false\">)</mo><mi>n</mi></msub><mo>⋯</mo><mo form=\"prefix\" stretchy=\"false\">(</mo><msub><mi>c</mi><mi>q</mi></msub><msub><mo form=\"postfix\" stretchy=\"false\">)</mo><mi>n</mi></msub></mrow></mfrac><mfrac><msup><mi>z</mi><mi>n</mi></msup><mrow><mi>n</mi><mo form=\"postfix\" lspace=\"0em\" rspace=\"0em\" stretchy=\"false\">!</mo></mrow></mfrac></mrow></math>",
        ),
        (
            r"S=dD\sin\alpha",
            "<math><mrow><mi>S</mi><mo>=</mo></mrow><mrow><mi>d</mi><mi>D</mi><mrow><mspace width=\"0.1667em\"></mspace><mi>sin</mi><mo>⁡</mo><mspace width=\"0.1667em\"></mspace></mrow><mi>α</mi></mrow></math>",
        ),
        (
            r"V = \frac{1}{6} \pi h \left [ 3 \left ( r_1^2 + r_2^2 \right ) + h^2 \right ]",
            "<math><mrow><mi>V</mi><mo>=</mo></mrow><mrow><mfrac><mn>1</mn><mn>6</mn></mfrac><mi>π</mi><mi>h</mi><mrow><mo fence=\"true\" form=\"prefix\">[</mo><mn>3</mn><mrow><mo fence=\"true\" form=\"prefix\">(</mo><msubsup><mi>r</mi><mn>1</mn><mn>2</mn></msubsup><mo>+</mo><msubsup><mi>r</mi><mn>2</mn><mn>2</mn></msubsup><mo fence=\"true\" form=\"postfix\">)</mo></mrow><mo>+</mo><msup><mi>h</mi><mn>2</mn></msup><mo fence=\"true\" form=\"postfix\">]</mo></mrow></mrow></math>",
        ),
    ];
    let mut n_match = 0usize;
    let mut n_diff = 0usize;
    let mut n_fail = 0usize;
    let converter = LatexToMathML::new(&MathCoreConfig::default()).unwrap();
    for (i, (latex, correct)) in problems.into_iter().enumerate() {
        let with_row = "{".to_string() + latex + "}";
        let mathml = converter.convert_with_local_counter(&with_row, MathDisplay::Inline);
        match mathml {
            Ok(mathml) => {
                if mathml != correct {
                    // println!("latex: {}", latex);
                    // let mathml = prettify(&mathml);
                    // let correct = prettify(correct);
                    // let diff = TextDiff::from_lines(&mathml, &correct);
                    // for change in diff.iter_all_changes() {
                    //     let sign = match change.tag() {
                    //         ChangeTag::Delete => "-",
                    //         ChangeTag::Insert => "+",
                    //         ChangeTag::Equal => " ",
                    //     };
                    //     print!("{}{}", sign, change);
                    // }
                    n_diff += 1;
                } else {
                    n_match += 1;
                }
            }
            Err(_e) => {
                // println!("latex: {}", latex);
                // println!("error: {}", e);
                println!("fail: {}", i);
                n_fail += 1;
            }
        }
    }
    assert_eq!(n_match, 10);
    assert_eq!(n_diff, 182);
    assert_eq!(n_fail, 26);
}

/// Prettify HTML input
pub fn prettify(input: &str) -> String {
    static OPEN_TAG: LazyLock<Regex> = LazyLock::new(|| Regex::new("(?P<tag><[A-z])").unwrap());

    // First get all tags on their own lines
    let mut stage1 = input.to_string();
    stage1 = stage1.replace("<!--", "\n<!--");
    stage1 = stage1.replace("-->", "-->\n");
    stage1 = stage1.replace("</", "\n</");
    stage1 = OPEN_TAG.replace_all(&stage1, "\n$tag").to_string();
    stage1 = stage1.trim().to_string();

    // Now fix indentation
    let mut stage2: Vec<String> = vec![];
    let mut indent = 0;
    for line in stage1.split('\n') {
        let mut post_add = 0;
        if line.starts_with("</") {
            indent -= 1;
        } else if line.ends_with("/>") || line.starts_with("<!DOCTYPE") {
            // Self-closing, nothing
            // or DOCTYPE, also nothing
        } else if line.starts_with('<') {
            post_add += 1;
        }

        stage2.push(format!("{}{}", "  ".repeat(indent), line));
        indent += post_add;
    }

    stage2.join("\n")
}

#[test]
fn test_nonfailing_wiki_tests() {
    let problems = [
        (0, r"\alpha"),
        (1, r"f(x) = x^2"),
        (2, r"\{1,e,\pi\}"),
        (3, r"|z + 1| \leq 2"),
        (4, r"\# \$ \% \wedge \& \_ \{ \} \sim \backslash"),
        (5, r"\dot{a}, \ddot{a}, \acute{a}, \grave{a}"),
        (6, r"\dot{a}, \ddot{a}, \acute{a}, \grave{a}"),
        (7, r"\check{a}, \breve{a}, \tilde{a}, \bar{a}"),
        (8, r"\hat{a}, \widehat{a}, \vec{a}"),
        (9, r"\exp_a b = a^b, \exp b = e^b, 10^m"),
        (10, r"\ln c, \lg d = \log e, \log_{10} f"),
        (11, r"\sin a, \cos b, \tan c, \cot d, \sec e, \csc f"),
        (12, r"\arcsin h, \arccos i, \arctan j"),
        (13, r"\sinh k, \cosh l, \tanh m, \coth n"),
        (
            14,
            r"\operatorname{sh}k, \operatorname{ch}l, \operatorname{th}m, \operatorname{coth}n",
        ),
        (15, r"\sgn r, \left\vert s \right\vert"),
        (16, r"\min(x,y), \max(x,y)"),
        (17, r"\min x, \max y, \inf s, \sup t"),
        (18, r"\lim u, \liminf v, \limsup w"),
        (19, r"\dim p, \deg q, \det m, \ker\phi"),
        (20, r"\Pr j, \hom l, \lVert z \rVert, \arg z"),
        (21, r"dt, \mathrm{d}t, \partial t, \nabla\psi"),
        (
            22,
            r"dy/dx, \mathrm{d}y/\mathrm{d}x, \frac{dy}{dx}, \frac{\mathrm{d}y}{\mathrm{d}x}, \frac{\partial^2} {\partial x_1\partial x_2}y",
        ),
        (
            23,
            r"\prime, \backprime, f^\prime, f', f'', f^{(3)}, \dot y, \ddot y",
        ),
        (
            24,
            r"\infty, \aleph, \complement,\backepsilon, \eth, \Finv, \hbar",
        ),
        (
            25,
            r"\Im, \imath, \jmath, \Bbbk, \ell, \mho, \wp, \Re, \circledS, \S, \P, \text\AA",
        ),
        (26, r"s_k \equiv 0 \pmod{m}"),
        (27, r"a \bmod b"),
        (28, r"\gcd(m, n), \operatorname{lcm}(m, n)"),
        // (29, r"\mid, \nmid, \shortmid, \nshortmid"),
        // (30, r"\surd, \sqrt{2}, \sqrt[n]{2}, \sqrt[3]{\frac{x^3+y^3}{2}}"),
        (31, r"+, -, \pm, \mp, \dotplus"),
        (32, r"\times, \div, \divideontimes, /, \backslash"),
        (33, r"\cdot, * \ast, \star, \circ, \bullet"),
        (34, r"\boxplus, \boxminus, \boxtimes, \boxdot"),
        (35, r"\oplus, \ominus, \otimes, \oslash, \odot"),
        (36, r"\circleddash, \circledcirc, \circledast"),
        (37, r"\bigoplus, \bigotimes, \bigodot"),
        (38, r"\{ \}, \text\O \empty \emptyset, \varnothing"),
        (39, r"\in, \notin \not\in, \ni, \not\ni"),
        (40, r"\cap, \Cap, \sqcap, \bigcap"),
        (
            41,
            r"\cup, \Cup, \sqcup, \bigcup, \bigsqcup, \uplus, \biguplus",
        ),
        // (42, r"\setminus, \smallsetminus, \times"),
        (43, r"\subset, \Subset, \sqsubset"),
        (44, r"\supset, \Supset, \sqsupset"),
        // (45, r"\subseteq, \nsubseteq, \subsetneq, \varsubsetneq, \sqsubseteq"),
        // (46, r"\supseteq, \nsupseteq, \supsetneq, \varsupsetneq, \sqsupseteq"),
        // (47, r"\subseteqq, \nsubseteqq, \subsetneqq, \varsubsetneqq"),
        // (48, r"\supseteqq, \nsupseteqq, \supsetneqq, \varsupsetneqq"),
        (49, r"=, \ne, \neq, \equiv, \not\equiv"),
        (
            50,
            r"\doteq, \doteqdot, \overset{\underset{\mathrm{def}}{}}{=}, :=",
        ),
        // (51, r"\sim, \nsim, \backsim, \thicksim, \simeq, \backsimeq, \eqsim, \cong, \ncong"),
        // (52, r"\approx, \thickapprox, \approxeq, \asymp, \propto, \varpropto"),
        (53, r"<, \nless, \ll, \not\ll, \lll, \not\lll, \lessdot"),
        // (54, r"\le, \leq, \lneq, \leqq, \nleq, \nleqq, \lneqq, \lvertneqq"),
        // (55, r"\ge, \geq, \gneq, \geqq, \ngeq, \ngeqq, \gneqq, \gvertneqq"),
        (
            56,
            r"\lessgtr, \lesseqgtr, \lesseqqgtr, \gtrless, \gtreqless, \gtreqqless",
        ),
        // (57, r"\leqslant, \nleqslant, \eqslantless"),
        // (58, r"\geqslant, \ngeqslant, \eqslantgtr"),
        (59, r"\lesssim, \lnsim, \lessapprox, \lnapprox"),
        (60, r"\gtrsim, \gnsim, \gtrapprox, \gnapprox"),
        (61, r"\prec, \nprec, \preceq, \npreceq, \precneqq"),
        (62, r"\succ, \nsucc, \succeq, \nsucceq, \succneqq"),
        (63, r"\preccurlyeq, \curlyeqprec"),
        (64, r"\succcurlyeq, \curlyeqsucc"),
        (65, r"\precsim, \precnsim, \precapprox, \precnapprox"),
        (66, r"\succsim, \succnsim, \succapprox, \succnapprox"),
        // (67, r"\parallel, \nparallel, \shortparallel, \nshortparallel"),
        (
            68,
            r"\perp, \angle, \sphericalangle, \measuredangle, 45^\circ",
        ),
        (
            69,
            r"\Box, \square, \blacksquare, \diamond, \Diamond, \lozenge, \blacklozenge,\bigstar",
        ),
        (70, r"\bigcirc, \triangle, \bigtriangleup, \bigtriangledown"),
        (71, r"\vartriangle, \triangledown"),
        (
            72,
            r"\blacktriangle, \blacktriangledown, \blacktriangleleft, \blacktriangleright",
        ),
        (73, r"\forall, \exists, \nexists"),
        (74, r"\therefore, \because, \And"),
        (75, r"\lor \vee, \curlyvee, \bigvee"),
        (76, r"\land \wedge, \curlywedge, \bigwedge"),
        // (77, r"\bar{q}, \bar{abc}, \overline{q}, \overline{abc}, \\ \lnot \neg, \not\operatorname{R},\bot,\top"),
        (78, r"\vdash \dashv, \vDash, \Vdash, \models"),
        (79, r"\Vvdash \nvdash \nVdash \nvDash \nVDash"),
        (80, r"\ulcorner \urcorner \llcorner \lrcorner"),
        (81, r"\Rrightarrow, \Lleftarrow"),
        (82, r"\Rightarrow, \nRightarrow, \Longrightarrow, \implies"),
        (83, r"\Leftarrow, \nLeftarrow, \Longleftarrow"),
        (
            84,
            r"\Leftrightarrow, \nLeftrightarrow, \Longleftrightarrow, \iff",
        ),
        (85, r"\Uparrow, \Downarrow, \Updownarrow"),
        (86, r"\rightarrow \to, \nrightarrow, \longrightarrow"),
        (87, r"\leftarrow \gets, \nleftarrow, \longleftarrow"),
        (
            88,
            r"\leftrightarrow, \nleftrightarrow, \longleftrightarrow",
        ),
        (89, r"\uparrow, \downarrow, \updownarrow"),
        (90, r"\nearrow, \swarrow, \nwarrow, \searrow"),
        (91, r"\mapsto, \longmapsto"),
        (
            92,
            r"\rightharpoonup \rightharpoondown \leftharpoonup \leftharpoondown \upharpoonleft \upharpoonright \downharpoonleft \downharpoonright \rightleftharpoons \leftrightharpoons",
        ),
        (
            93,
            r"\curvearrowleft \circlearrowleft \Lsh \upuparrows \rightrightarrows \rightleftarrows \rightarrowtail \looparrowright",
        ),
        (
            94,
            r"\curvearrowright \circlearrowright \Rsh \downdownarrows \leftleftarrows \leftrightarrows \leftarrowtail \looparrowleft",
        ),
        (
            95,
            r"\hookrightarrow \hookleftarrow \multimap \leftrightsquigarrow \rightsquigarrow \twoheadrightarrow \twoheadleftarrow",
        ),
        (96, r"\amalg \P \S \% \dagger\ddagger\ldots\cdots"),
        (97, r"\smile \frown \wr \triangleleft \triangleright"),
        (
            98,
            r"\diamondsuit, \heartsuit, \clubsuit, \spadesuit, \Game, \flat, \natural, \sharp",
        ),
        // (99, r"\diagup \diagdown \centerdot \ltimes \rtimes \leftthreetimes \rightthreetimes"),
        (
            100,
            r"\eqcirc \circeq \triangleq \bumpeq\Bumpeq \doteqdot \risingdotseq \fallingdotseq",
        ),
        (
            101,
            r"\intercal \barwedge \veebar \doublebarwedge \between \pitchfork",
        ),
        // (102, r"\vartriangleleft \ntriangleleft \vartriangleright \ntriangleright"),
        // (103, r"\trianglelefteq \ntrianglelefteq \trianglerighteq \ntrianglerighteq"),
        (104, r"a^2, a^{x+3}"),
        (105, r"a_2"),
        (106, r"10^{30} a^{2+2} \\ a_{i,j} b_{f'}"),
        (107, r"x_2^3 \\ {x_2}^3"),
        (108, r"10^{10^{8}}"),
        (
            109,
            r"\overset{\alpha}{\omega} \\ \underset{\alpha}{\omega} \\ \overset{\alpha}{\underset{\gamma}{\omega}}\\ \stackrel{\alpha}{\omega}",
        ),
        (110, r"x', y'', f', f'' \\ x^\prime, y^{\prime\prime}"),
        (111, r"\dot{x}, \ddot{x}"),
        (
            112,
            r"\hat a \ \bar b \ \vec c \\ \overrightarrow{a b} \ \overleftarrow{c d}\\ \widehat{d e f} \\ \overline{g h i} \ \underline{j k l}",
        ),
        (113, r"\overset{\frown} {AB}"),
        // (114, r"A \xleftarrow{n+\mu-1} B \xrightarrow[T]{n\pm i-1} C"),
        (115, r"\overbrace{ 1+2+\cdots+100 }^{5050}"),
        (116, r"\underbrace{ a+b+\cdots+z }_{26}"),
        (117, r"\frac{2}{4}=0.5"),
        (
            118,
            r"\dfrac{2}{4} = 0.5 \qquad \dfrac{2}{c + \dfrac{2}{d + \dfrac{2}{4}}} = a",
        ),
        // (119, r"\cfrac{x}{1 + \cfrac{\cancel{y}} {\cancel{y}}} = \cfrac{x}{2}"),
        (120, r"\binom{n}{k}"),
        (121, r"\dbinom{n}{k}"),
        (122, r"\begin{matrix} x & y \\ z & v \end{matrix}"),
        (123, r"\begin{vmatrix} x & y \\ z & v \end{vmatrix}"),
        (124, r"\begin{Vmatrix} x & y \\ z & v \end{Vmatrix}"),
        (
            125,
            r"\begin{bmatrix} 0 & \cdots & 0 \\ \vdots & \ddots & \vdots \\ 0 & \cdots & 0 \end{bmatrix}",
        ),
        (126, r"\begin{Bmatrix} x & y \\ z & v \end{Bmatrix}"),
        (127, r"\begin{pmatrix} x & y \\ z & v \end{pmatrix}"),
        // (128, r"\bigl( \begin{smallmatrix} a&b\\ c&d \end{smallmatrix} \bigr)"),
        (
            129,
            r"f(n) = \begin{cases} n/2, & \text{if }n\text{ is even} \\ 3n+1, & \text{if }n\text{ is odd} \end{cases}",
        ),
        (
            130,
            r"\begin{cases} 3x + 5y + z \\ 7x - 2y + 4z \\ -6x + 3y + 2z \end{cases}",
        ),
        (131, r"f(x) \,\!"),
        // (132, r"\begin{array}{|c|c|c|} a & b & S \\ \hline 0 & 0 & 1 \\ 0 & 1 & 1 \\ 1 & 0 & 1 \\ 1 & 1 & 0 \\ \end{array}"),
        (133, r"( \frac{1}{2} )^n"),
        (134, r"\left ( \frac{1}{2} \right )^n"),
        (135, r"\left ( \frac{a}{b} \right )"),
        (
            136,
            r"\left [ \frac{a}{b} \right ] \quad \left \lbrack \frac{a}{b} \right \rbrack",
        ),
        (
            137,
            r"\left \{ \frac{a}{b} \right \} \quad \left \lbrace \frac{a}{b} \right \rbrace",
        ),
        (138, r"\left \langle \frac{a}{b} \right \rangle"),
        (
            139,
            r"\left | \frac{a}{b} \right \vert \quad \left \Vert \frac{c}{d} \right \|",
        ),
        (
            140,
            r"\left \lfloor \frac{a}{b} \right \rfloor \quad \left \lceil \frac{c}{d} \right \rceil",
        ),
        (141, r"\left / \frac{a}{b} \right \backslash"),
        (
            142,
            r"\left\uparrow\frac{a}{b}\right\downarrow\; \left\Uparrow\frac{a}{b}\right\Downarrow\; \left \updownarrow \frac{a}{b} \right \Updownarrow",
        ),
        (143, r"\left [ 0,1 \right ) \left \langle \psi \right |"),
        (144, r"\left . \frac{A}{B} \right \} \to X"),
        (
            145,
            r"( \bigl( \Bigl( \biggl( \Biggl( \dots \Biggr] \biggr] \Bigr] \bigr] ]",
        ),
        (
            146,
            r"\{ \bigl\{ \Bigl\{ \biggl\{ \Biggl\{ \dots \Biggr\rangle \biggr\rangle \Bigr\rangle \bigr\rangle \rangle",
        ),
        (
            147,
            r"\| \big\| \Big\| \bigg\| \Bigg\| \dots \Bigg| \bigg| \Big| \big| |",
        ),
        (
            148,
            r"\lfloor \bigl\lfloor \Bigl\lfloor \biggl\lfloor \Biggl\lfloor \dots \Biggr\rceil \biggr\rceil \Bigr\rceil \bigr\rceil \rceil",
        ),
        (
            149,
            r"\uparrow \big\uparrow \Big\uparrow \bigg\uparrow \Bigg\uparrow \dots \Bigg\Downarrow \bigg\Downarrow \Big\Downarrow \big\Downarrow \Downarrow",
        ),
        (
            150,
            r"\updownarrow\big\updownarrow\Big\updownarrow \bigg\updownarrow \Bigg\updownarrow \dots \Bigg\Updownarrow \bigg\Updownarrow \Big \Updownarrow \big\Updownarrow \Updownarrow",
        ),
        (
            151,
            r"/ \big/ \Big/ \bigg/ \Bigg/ \dots \Bigg\backslash \bigg\backslash \Big \backslash \big\backslash \backslash",
        ),
        (
            152,
            r"\Alpha \Beta \Gamma \Delta \Epsilon \Zeta \Eta \Theta",
        ),
        (153, r"\Iota \Kappa \Lambda \Mu \Nu \Xi \Omicron \Pi"),
        (154, r"\Rho \Sigma \Tau \Upsilon \Phi \Chi \Psi \Omega"),
        (
            155,
            r"\alpha \beta \gamma \delta \epsilon \zeta \eta \theta",
        ),
        (156, r"\iota \kappa \lambda \mu \nu \xi \omicron \pi"),
        (157, r"\rho \sigma \tau \upsilon \phi \chi \psi \omega"),
        (
            158,
            r"\varGamma \varDelta \varTheta \varLambda \varXi \varPi \varSigma \varPhi \varUpsilon \varOmega",
        ),
        (
            159,
            r"\varepsilon \digamma \varkappa \varpi \varrho \varsigma \vartheta \varphi",
        ),
        (160, r"\aleph \beth \gimel \daleth"),
        (
            161,
            r"\mathbb{ABCDEFGHI} \\ \mathbb{JKLMNOPQR} \\ \mathbb{STUVWXYZ}",
        ),
        (
            162,
            r"\mathbf{ABCDEFGHI} \\ \mathbf{JKLMNOPQR} \\ \mathbf{STUVWXYZ} \\ \mathbf{abcdefghijklm} \\ \mathbf{nopqrstuvwxyz} \\ \mathbf{0123456789}",
        ),
        (
            163,
            r"\boldsymbol{\Alpha \Beta \Gamma \Delta \Epsilon \Zeta \Eta \Theta}",
        ),
        (
            164,
            r"\boldsymbol{\Iota \Kappa \Lambda \Mu \Nu \Xi \Omicron \Pi}",
        ),
        (
            165,
            r"\boldsymbol{\Rho \Sigma \Tau \Upsilon \Phi \Chi \Psi \Omega}",
        ),
        (
            166,
            r"\boldsymbol{\alpha \beta \gamma \delta \epsilon \zeta \eta \theta}",
        ),
        (
            167,
            r"\boldsymbol{\iota \kappa \lambda \mu \nu \xi \omicron \pi}",
        ),
        (
            168,
            r"\boldsymbol{\rho \sigma \tau \upsilon \phi \chi \psi \omega}",
        ),
        (169, r"\boldsymbol{\varepsilon\digamma\varkappa \varpi}"),
        (170, r"\boldsymbol{\varrho\varsigma\vartheta\varphi}"),
        (171, r"\mathit{0123456789}"),
        (
            172,
            r"\mathit{\Alpha \Beta \Gamma \Delta \Epsilon \Zeta \Eta \Theta}",
        ),
        (
            173,
            r"\mathit{\Iota \Kappa \Lambda \Mu \Nu \Xi \Omicron \Pi}",
        ),
        (
            174,
            r"\mathit{\Rho \Sigma \Tau \Upsilon \Phi \Chi \Psi \Omega}",
        ),
        (
            175,
            r"\boldsymbol{\varGamma \varDelta \varTheta \varLambda}",
        ),
        (
            176,
            r"\boldsymbol{\varXi \varPi \varSigma \varUpsilon \varOmega}",
        ),
        (
            177,
            r"\mathrm{ABCDEFGHI} \\ \mathrm{JKLMNOPQR} \\ \mathrm{STUVWXYZ} \\ \mathrm{abcdefghijklm} \\ \mathrm{nopqrstuvwxyz} \\ \mathrm{0123456789}",
        ),
        (
            178,
            r"\mathsf{ABCDEFGHI} \\ \mathsf{JKLMNOPQR} \\ \mathsf{STUVWXYZ} \\ \mathsf{abcdefghijklm} \\ \mathsf{nopqrstuvwxyz} \\ \mathsf{0123456789}",
        ),
        (
            179,
            r"\mathsf{\Alpha \Beta \Gamma \Delta \Epsilon \Zeta \Eta \Theta}",
        ),
        (
            180,
            r"\mathsf{\Iota \Kappa \Lambda \Mu \Nu \Xi \Omicron \Pi}",
        ),
        (
            181,
            r"\mathsf{\Rho \Sigma \Tau \Upsilon \Phi \Chi \Psi \Omega}",
        ),
        (
            182,
            r"\mathcal{ABCDEFGHI} \\ \mathcal{JKLMNOPQR} \\ \mathcal{STUVWXYZ} \\ \mathcal{abcdefghi} \\ \mathcal{jklmnopqr} \\ \mathcal{stuvwxyz}",
        ),
        (
            183,
            r"\mathfrak{ABCDEFGHI} \\ \mathfrak{JKLMNOPQR} \\ \mathfrak{STUVWXYZ} \\ \mathfrak{abcdefghi} \\ \mathfrak{jklmnopqr} \\ \mathfrak{stuvwxyz}",
        ),
        (184, r"{\scriptstyle\text{abcdefghijklm}}"),
        (185, r"x y z"),
        (186, r"\text{x y z}"),
        (187, r"\text{if} n \text{is even}"),
        (188, r"\text{if }n\text{ is even}"),
        (189, r"\text{if}~n\ \text{is even}"),
        (
            190,
            r"{\color{Blue}x^2}+{\color{Orange}2x}- {\color{LimeGreen}1}",
        ),
        (
            191,
            r"x_{1,2}=\frac{{\color{Blue}-b}\pm \sqrt{\color{Red}b^2-4ac}}{\color{Green}2a }",
        ),
        (
            192,
            r"{\color{Blue}x^2}+{\color{Orange}2x}- {\color{LimeGreen}1}",
        ),
        (
            193,
            r"\color{Blue}x^2\color{Black}+\color{Orange} 2x\color{Black}-\color{LimeGreen}1",
        ),
        (
            194,
            r"\color{Blue}{x^2}+\color{Orange}{2x}- \color{LimeGreen}{1}",
        ),
        // (195, r"\definecolor{myorange}{rgb}{1,0.65,0.4} \color{myorange}e^{i \pi}\color{Black} + 1= 0"),
        (
            196,
            r"a \qquad b \\ a \quad b \\ a\ b \\ a \text{ } b \\ a\;b \\ a\,b \\ ab \\ a b \\ \mathit{ab} \\ a\!b",
        ),
        (197, r"| \uparrow \rangle"),
        (198, r"\left| \uparrow \right\rangle"),
        (199, r"| {\uparrow} \rangle"),
        // (200, r"| \mathord\uparrow \rangle"),
        (201, r"\wideparen{AB}"),
        (202, r"\dddot{x}"),
        // (203, r"\sout{q}"),
        // (204, r"\mathrlap{\,/}{=}"),
        // (205, r"\text{\textsf{textual description}}"),
        (206, r"α π"),
        (207, r"ax^2 + bx + c = 0"),
        (208, r"x=\frac{-b\pm\sqrt{b^2-4ac}}{2a}"),
        (209, r"\left( \frac{\left(3-x\right) \times 2}{3-x} \right)"),
        (210, r"\det(\mathsf{A}-\lambda\mathsf{I}) = 0"),
        (211, r"u'' + p(x)u' + q(x)u=f(x),\quad x>a"),
        (
            212,
            r"|\bar{z}| = |z|, |(\bar{z})^n| = |z|^n, \arg(z^n) = n \arg(z)",
        ),
        (
            213,
            r"\phi_n(\kappa) = 0.033C_n^2\kappa^{-11/3}, \quad\frac{1}{L_0}\ll\kappa\ll\frac{1}{l_0}",
        ),
        (
            214,
            r"f(x) = \begin{cases} 1 & -1 \le x < 0 \\ \frac{1}{2} & x = 0 \\ 1 - x^2 & \text{otherwise} \end{cases}",
        ),
        (
            215,
            r"{}_pF_q(a_1,\dots,a_p;c_1,\dots,c_q;z) = \sum_{n=0}^\infty \frac{(a_1)_n\cdots(a_p)_n} {(c_1)_n\cdots(c_q)_n}\frac{z^n}{n!}",
        ),
        (216, r"S=dD\sin\alpha"),
        (
            217,
            r"V = \frac{1}{6} \pi h \left [ 3 \left ( r_1^2 + r_2^2 \right ) + h^2 \right ]",
        ),
    ];

    let converter = LatexToMathML::new(&MathCoreConfig {
        pretty_print: PrettyPrint::Always,
        ..Default::default()
    })
    .unwrap();
    for (num, problem) in problems.into_iter() {
        let mathml = converter
            .convert_with_local_counter(problem, crate::MathDisplay::Inline)
            .expect(format!("failed to convert `{}`", problem).as_str());
        let name = format!("wiki{:03}", num);
        assert_snapshot!(name.as_str(), &mathml, problem);
    }
}
