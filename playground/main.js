import { convert } from "./pkg/math_core_wasm.js";

// Global cached values
let cachedIsBlock = true; // default value
let cachedConfig = { pretty: true, macros: new Map() }; // default value

function initializeCachedValues() {
  // Initialize cached values based on current DOM state
  const blockRadio = document.querySelector(
    '#displaystyle input[type="radio"]:checked',
  );
  cachedIsBlock = blockRadio ? blockRadio.value === "block" : true;

  const prettyRadio = document.querySelector(
    '#prettyprint input[type="radio"]:checked',
  );
  let macros = new Map();
  macros.set("half", "\\frac{1}{2}");
  cachedConfig = {
    pretty: prettyRadio ? prettyRadio.value === "true" : true,
    macros,
  };
}

function updateIsBlockCache() {
  const selectedRadio = document.querySelector(
    '#displaystyle input[type="radio"]:checked',
  );
  cachedIsBlock = selectedRadio ? selectedRadio.value === "block" : true;
}

function updateConfigCache() {
  const selectedRadio = document.querySelector(
    '#prettyprint input[type="radio"]:checked',
  );
  cachedConfig = {
    pretty: selectedRadio ? selectedRadio.value === "true" : true,
    macros: {},
  };
}

function isBlock() {
  return cachedIsBlock;
}

function config() {
  return cachedConfig;
}

document.addEventListener("DOMContentLoaded", () => {
  // Initialize cached values on page load
  initializeCachedValues();

  const inputField = document.getElementById("inputField");
  const outputField = document.getElementById("outputField");
  const outputCode = document.getElementById("outputCode");

  function updateOutput() {
    try {
      const input = inputField.value;
      const output = convert(input, isBlock(), config());
      outputField.innerHTML = output;
      outputCode.textContent = output;
    } catch (error) {
      outputField.innerHTML = "";
      outputCode.textContent = `Error at location ${error.location}: ${error.error_message}`;
    }
  }

  inputField.addEventListener("input", () => {
    updateOutput();
  });

  document
    .querySelectorAll('#displaystyle input[type="radio"]')
    .forEach((radio) => {
      radio.addEventListener("change", () => {
        updateIsBlockCache(); // Update cache when radio button changes
        updateOutput();
      });
    });

  document
    .querySelectorAll('#prettyprint input[type="radio"]')
    .forEach((radio) => {
      radio.addEventListener("change", () => {
        updateConfigCache(); // Update cache when radio button changes
        updateOutput();
      });
    });

  const fontSelect = document.getElementById("math-font");
  const styleElement = document.getElementById("math-font-style");
  const fontFeaturesMap = {
    "Libertinus Math Regular": '"ss09"',
  };

  /* const mathBBMap = {
        'STIX Two Math Regular': 'TeX Gyre Pagella Math BB',
        'Latin Modern Math': 'TeX Gyre Pagella Math BB',
    }; */

  // Update the style rule when selection changes
  fontSelect.addEventListener("change", function () {
    const featureSettings = fontFeaturesMap[this.value]
      ? `font-feature-settings: ${fontFeaturesMap[this.value]};`
      : "";
    /* const mathBB = mathBBMap[this.value]
            ? `"${mathBBMap[this.value]}", `
            : ''; */
    styleElement.textContent = `math { font-family: "${this.value}", math; ${featureSettings} }`;
  });
});
