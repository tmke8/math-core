import init, { convert, set_config } from "./pkg/math_core_wasm.js";

// Global cached values
let cachedIsBlock = true; // default value

function initializeCachedValues() {
  // Initialize cached values based on current DOM state
  const blockRadio = document.querySelector(
    '#displaystyle input[type="radio"]:checked',
  );
  cachedIsBlock = blockRadio ? blockRadio.value === "block" : true;
}

function updateIsBlockCache() {
  const selectedRadio = document.querySelector(
    '#displaystyle input[type="radio"]:checked',
  );
  cachedIsBlock = selectedRadio ? selectedRadio.value === "block" : true;
}

function updateConfig() {
  const configField = document.getElementById("configField");
  
  try {
    const jsonString = configField.value.trim();
    
    // Parse JSON with custom reviver to convert macros to Map
    const parsed = JSON.parse(jsonString, (key, value) => {
      if (key === 'macros') {
        return new Map(Object.entries(value));
      }
      return value;
    });
    set_config(parsed);
    
  } catch (error) {
    const outputCode = document.getElementById("outputCode");
    if (outputCode) {
      outputCode.textContent = `Error parsing config: ${error.message}`;
    }
    return false; // Return false to indicate failure
  }
  return true;
}

function isBlock() {
  return cachedIsBlock;
}

document.addEventListener("DOMContentLoaded", () => {
  init().then(() => {
    console.log("WASM module initialized");
    updateConfig(); // Initial config setup
  });

  // Initialize cached values on page load
  initializeCachedValues();

  const inputField = document.getElementById("inputField");
  const outputField = document.getElementById("outputField");
  const outputCode = document.getElementById("outputCode");
  const configField = document.getElementById("configField");

  function updateOutput() {
    try {
      const input = inputField.value;
      const output = convert(input, isBlock());
      outputField.innerHTML = output;
      outputCode.textContent = output;
    } catch (error) {
      outputField.innerHTML = "";
      outputCode.textContent = `Error at location ${error.location}: ${error.message}`;
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

  // Add listener for config field changes
  configField.addEventListener("input", () => {
    const success = updateConfig(); // Update config when textarea content changes
    if (success) {
      updateOutput(); // Re-render output with new config
    } else {
      outputField.innerHTML = "";
    }
  });

  const fontSelect = document.getElementById("math-font");
  const styleElement = document.getElementById("math-font-style");
  const fontFeaturesMap = {
    "Libertinus Math Regular": '"ss09"',
  };

  // Update the style rule when selection changes
  fontSelect.addEventListener("change", function () {
    const featureSettings = fontFeaturesMap[this.value]
      ? `font-feature-settings: ${fontFeaturesMap[this.value]};`
      : "";
    styleElement.textContent = `math { font-family: "${this.value}", math; ${featureSettings} }`;
  });
});
