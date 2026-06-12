import { LatexToMathML } from "../dist/math_core.js";
import { assert } from "chai";

describe("Convert Tests", function () {
  context("Simple commands", function () {
    const converter = new LatexToMathML({});
    it("should convert simple command correctly", function () {
      const latex = "x\\sum x";
      const displayStyle = false;
      assert.equal(
        converter.convert_with_local_state(latex, displayStyle),
        "<math><mi>x</mi><mo>∑</mo><mi>x</mi></math>",
      );
    });
    it("should convert simple command correctly in display style", function () {
      const latex = "x\\sum x";
      const displayStyle = true;
      assert.equal(
        converter.convert_with_local_state(latex, displayStyle),
        '<math display="block"><mi>x</mi><mo>∑</mo><mi>x</mi></math>',
      );
    });
  });
  context("Local equation numbering", function () {
    it("should convert equation with local numbering", function () {
      const converter = new LatexToMathML({
        prettyPrint: "auto",
      });
      const latex = "\\begin{align}x\\\\y\\end{align}";
      const displayStyle = true;
      const output = converter.convert_with_local_state(latex, displayStyle);
      assert.include(output, "(1)");
      assert.include(output, "(2)");
      const output2 = converter.convert_with_local_state(latex, displayStyle);
      assert.include(output2, "(1)");
      assert.include(output2, "(2)");
    });
  });
  context("Global equation numbering", function () {
    const converter = new LatexToMathML({
      prettyPrint: "auto",
    });
    it("should convert equation with global numbering", function () {
      const latex = "\\begin{align}x\\\\y\\end{align}";
      const displayStyle = true;
      const output = converter.convert_with_global_state(latex, displayStyle);
      assert.include(output, "(1)");
      assert.include(output, "(2)");
      const output2 = converter.convert_with_global_state(
        latex,
        displayStyle,
      );
      assert.include(output2, "(3)");
      assert.include(output2, "(4)");
    });
    it("should convert equation with global numbering, the second time", function () {
      converter.reset_global_state();
      const latex = "\\begin{align}x\\\\y\\end{align}";
      const displayStyle = true;
      const output = converter.convert_with_global_state(latex, displayStyle);
      assert.include(output, "(1)");
      assert.include(output, "(2)");
      const output2 = converter.convert_with_global_state(
        latex,
        displayStyle,
      );
      assert.include(output2, "(3)");
      assert.include(output2, "(4)");
    });
  });
  context("Macros", function () {
    it("should convert with custom macros", function () {
      const macros = new Map();
      macros.set("RR", "\\mathbb{R}");
      const converter = new LatexToMathML({
        macros,
      });
      const latex = "\\RR";
      const displayStyle = false;
      assert.equal(
        converter.convert_with_local_state(latex, displayStyle),
        "<math><mi>ℝ</mi></math>",
      );
    });
    it("should throw an error on invalid macro definition", function () {
      const macros = new Map();
      macros.set("RR", "\\mathb{R}");
      // First assert that the error is thrown.
      assert.throws(() => {
        new LatexToMathML({
          macros,
        });
      });
      // Then assert that the error contains the correct context and location.
      try {
        new LatexToMathML({
          macros,
        });
      } catch (e) {
        assert.match(e.message, /Unknown command "\\mathb"./);
        assert.equal(e.label, "unknown command");
        assert.equal(e.context, "\\mathb{R}");
        assert.equal(e.start, 0);
        assert.equal(e.end, 6);
      }
    });
  });
  context("Config parse errors", function () {
    it("should throw an error on invalid prettyPrint value", function () {
      assert.throws(() => {
        new LatexToMathML({
          // @ts-expect-error
          prettyPrint: "sometimes",
        });
      }, /Invalid value for prettyPrint/);
    });
    it("should throw an error on invalid macro map", function () {
      const macros = new Map();
      macros.set("RR", 42);
      assert.throws(() => {
        new LatexToMathML({
          macros,
        });
      }, /Invalid macros map/);
    });
  });
  context("Continue on error", function () {
    it("should continue on error", function () {
      const converter = new LatexToMathML({
        throwOnError: false,
      });
      const latex = "𐌸\\asdf";
      const displayStyle = false;
      assert.equal(
        converter.convert_with_local_state(latex, displayStyle),
        '<span class="math-core-error" title="1: Unknown command &quot;\\asdf&quot;."><code>𐌸\\asdf</code></span>',
      );
    });
  });
  context("Throwing an error", function () {
    it("should throw on error", function () {
      const converter = new LatexToMathML({});
      const latex = "\\asdf";
      const displaystyle = false;
      assert.throws(() => {
        converter.convert_with_local_state(latex, displaystyle);
      }, /Unknown command "\\asdf"./);
    });
  });
  context("Showing error reports", function () {
    it("should show a report", function () {
      try {
        const converter = new LatexToMathML({});
        const latex = "\\begin{foobar}";
        const displaystyle = false;
        converter.convert_with_local_state(latex, displaystyle);
      } catch (e) {
        assert.equal(e.report, `Error: Unknown environment "foobar".
   ╭─[ input:1:7 ]
   │
 1 │ \\begin{foobar}
   │       ────┬───  
   │           ╰───── unknown environment
───╯
`);
      }
    });
  });
});
