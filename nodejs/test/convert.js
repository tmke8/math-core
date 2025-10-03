import { LatexToMathML } from "../dist/math_core.js";
import { assert } from "chai";

describe("Convert Tests", function () {
  context("Simple command", function () {
    it("should convert simple command correctly", function () {
      const converter = new LatexToMathML({
        prettyPrint: "never",
        xmlNamespace: false,
        macros: new Map(),
      });
      const latex = "x\\sum x";
      const displayStyle = false;
      assert.equal(
        converter.convert_with_local_counter(latex, displayStyle),
        "<math><mi>x</mi><mo>∑</mo><mi>x</mi></math>",
      );
    });
  });
});
