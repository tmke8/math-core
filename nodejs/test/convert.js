import { LatexToMathML } from "../dist/math_core.js";
import { assert } from "chai";

describe("Convert Tests", function () {
  context("Simple commands", function () {
    const converter = new LatexToMathML({});
    it("should convert simple command correctly", function () {
      const latex = "x\\sum x";
      const displayStyle = false;
      assert.equal(
        converter.convert_with_local_counter(latex, displayStyle),
        "<math><mi>x</mi><mo>∑</mo><mi>x</mi></math>",
      );
    });
    it("should convert simple command correctly in display style", function () {
      const latex = "x\\sum x";
      const displayStyle = true;
      assert.equal(
        converter.convert_with_local_counter(latex, displayStyle),
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
      const output = converter.convert_with_local_counter(latex, displayStyle);
      assert.include(output, "(1)");
      assert.include(output, "(2)");
      const output2 = converter.convert_with_local_counter(latex, displayStyle);
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
      const output = converter.convert_with_global_counter(latex, displayStyle);
      assert.include(output, "(1)");
      assert.include(output, "(2)");
      const output2 = converter.convert_with_global_counter(
        latex,
        displayStyle,
      );
      assert.include(output2, "(3)");
      assert.include(output2, "(4)");
    });
    it("should convert equation with global numbering, the second time", function () {
      converter.reset_global_counter();
      const latex = "\\begin{align}x\\\\y\\end{align}";
      const displayStyle = true;
      const output = converter.convert_with_global_counter(latex, displayStyle);
      assert.include(output, "(1)");
      assert.include(output, "(2)");
      const output2 = converter.convert_with_global_counter(
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
        converter.convert_with_local_counter(latex, displayStyle),
        "<math><mi>ℝ</mi></math>",
      );
    });
  });
  context("Continue on error", function () {
    it("should continue on error", function () {
      const converter = new LatexToMathML({
        continueOnError: true,
      });
      const latex = "\\asdf";
      const displayStyle = false;
      assert.equal(
        converter.convert_with_local_counter(latex, displayStyle),
        '<span class="math-core-error" title="0: Unknown command &quot;\\asdf&quot;."><code>\\asdf</code></span>',
      );
    });
  });
});
