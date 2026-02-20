import init, { LatexToMathML } from "./pkg/math_core_wasm.js";

// Global cached values
let cachedIsBlock = true; // default value
/**
 * @type {LatexToMathML | null}
 */
let cachedConverter = null;

function initializeCachedValues() {
  // Initialize cached values based on current DOM state
  const blockRadio = document.querySelector(
    '#displaystyle input[type="radio"]:checked',
  );
  console.assert(blockRadio instanceof HTMLInputElement || blockRadio === null);
  cachedIsBlock = blockRadio ? blockRadio.value === "block" : true;
}

function updateIsBlockCache() {
  const selectedRadio = document.querySelector(
    '#displaystyle input[type="radio"]:checked',
  );
  cachedIsBlock = selectedRadio ? selectedRadio.value === "block" : true;
}

/**
 * Updates the cachedConverter based on the current config field and pretty print setting
 * @return {boolean} True if the config was successfully updated, false otherwise
 */
function updateConfig() {
  const prettyRadio = document.querySelector(
    '#prettyprint input[type="radio"]:checked',
  );
  const isPrettyPrint = prettyRadio ? prettyRadio.value === "true" : true;

  const configField = document.getElementById("configField");

  try {
    const jsonString = configField.value.trim();

    // Parse JSON with custom reviver to convert macros to Map
    let parsed = JSON.parse(jsonString, (key, value) => {
      if (key === "macros") {
        return new Map(Object.entries(value));
      }
      return value;
    });
    // Set the prettyPrint property from the radio selection
    parsed["prettyPrint"] = isPrettyPrint ? "always" : "never";
    cachedConverter = new LatexToMathML(parsed);
  } catch (error) {
    const outputCode = document.getElementById("outputCode");
    if (outputCode) {
      outputCode.textContent = `Error parsing config: ${error.message}`;
    }
    return false; // Return false to indicate failure
  }
  return true;
}

/**
 * Returns whether the current display mode is block or inline
 * @returns {boolean} True if block mode, false if inline
 */
function isBlock() {
  return cachedIsBlock;
}

async function generateLink() {
  const inputField = document.getElementById("inputField");
  console.assert(inputField instanceof HTMLTextAreaElement);
  const content = inputField.value;

  if (content.trim() === "") {
    alert("Please enter some text first!");
    return;
  }

  // Compress the content
  const compressedContent = await compressText(content);

  // Encode content to base64
  const encodedContent = uint8ArrayToBase64Url(compressedContent);

  // Generate the URL
  const currentUrl = window.location.origin + window.location.pathname;
  const generatedUrl = `${currentUrl}#input:${encodedContent}`;

  // Display the link
  document.getElementById("generatedLink").innerText = generatedUrl;
  document.getElementById("linkContainer").classList.remove("hidden");
}

// Function to load content from URL hash
async function loadFromUrl() {
  const hash = window.location.hash;

  if (hash.startsWith("#input:")) {
    const encodedContent = hash.substring(7); // Remove '#input:' prefix

    try {
      const compressedContent = base64UrlToUint8Array(encodedContent);
      const decodedContent = await decompressText(compressedContent);
      const inputField = document.getElementById("inputField");
      console.assert(inputField instanceof HTMLTextAreaElement);
      inputField.value = decodedContent;
      // Trigger input event to update output
      inputField.dispatchEvent(new Event("input", { bubbles: true }));

      // Clear the hash from URL without page reload
      history.replaceState(
        null,
        null,
        window.location.pathname + window.location.search,
      );
    } catch (err) {
      console.error("Failed to decode content from URL:", err);
      // Silently fail - invalid base64 in URL
    }
  }
}

/**
 * Compresses text using the Compression Streams API
 * @param {string} text - The text to compress
 * @returns {Promise<Uint8Array>} The compressed data
 */
async function compressText(text) {
  const format = "gzip";
  // Validate input
  if (typeof text !== "string") {
    throw new TypeError("Input must be a string");
  }

  // Convert text to Uint8Array
  const encoder = new TextEncoder();
  const data = encoder.encode(text);

  // Create a blob stream (more efficient than manual ReadableStream)
  const blob = new Blob([data]);
  const stream = blob.stream();

  // Compress the stream
  const compressedStream = stream.pipeThrough(new CompressionStream(format));

  // Use Response API for efficient reading (simpler than manual reader)
  const response = new Response(compressedStream);
  return new Uint8Array(await response.arrayBuffer());
}

/**
 * Decompresses data using the Compression Streams API
 * @param {Uint8Array} compressedData - The compressed data
 * @returns {Promise<string>} The decompressed text
 */
async function decompressText(compressedData) {
  const format = "gzip";
  // Validate input
  if (!(compressedData instanceof Uint8Array)) {
    throw new TypeError("Input must be a Uint8Array");
  }

  // Create a stream from compressed data
  const blob = new Blob([compressedData]);
  const stream = blob.stream();

  // Decompress the stream
  const decompressedStream = stream.pipeThrough(
    new DecompressionStream(format),
  );

  // Read and decode the result
  const response = new Response(decompressedStream);
  const arrayBuffer = await response.arrayBuffer();
  const decoder = new TextDecoder();
  return decoder.decode(arrayBuffer);
}

// Reference: https://phuoc.ng/collection/this-vs-that/concat-vs-push/
const MAX_BLOCK_SIZE = 65_535;

/**
 * Convert a Uint8Array to a base64url string
 * Uses native toBase64() when available, falls back to manual implementation otherwise
 * @param {Uint8Array} array - The array to convert
 * @returns {string} The base64url encoded string
 */
export function uint8ArrayToBase64Url(array) {
  if (!(array instanceof Uint8Array)) {
    throw new TypeError("Expected Uint8Array");
  }

  // Check if native method exists
  if (typeof array.toBase64 === "function") {
    return array.toBase64({ alphabet: "base64url" });
  }

  // Fallback implementation
  let base64;

  if (array.length < MAX_BLOCK_SIZE) {
    // Required as `btoa` and `atob` don't properly support Unicode: https://developer.mozilla.org/en-US/docs/Glossary/Base64#the_unicode_problem
    base64 = globalThis.btoa(String.fromCodePoint.apply(this, array));
  } else {
    base64 = "";
    for (const value of array) {
      base64 += String.fromCodePoint(value);
    }
    base64 = globalThis.btoa(base64);
  }

  return base64.replaceAll("+", "-").replaceAll("/", "_").replace(/=+$/, "");
}

/**
 * Convert a base64url string to a Uint8Array
 * Uses native fromBase64() when available, falls back to manual implementation
 * @param {string} base64String - The base64url string to convert
 * @returns {Uint8Array} The decoded array
 */
export function base64UrlToUint8Array(base64String) {
  if (typeof base64String !== "string") {
    throw new TypeError("Expected string");
  }

  // Check if native method exists
  if (typeof Uint8Array.fromBase64 === "function") {
    return Uint8Array.fromBase64(base64String, { alphabet: "base64url" });
  }

  // Fallback implementation - convert base64url to base64 first
  const base64 = base64String.replaceAll("-", "+").replaceAll("_", "/");
  return Uint8Array.from(globalThis.atob(base64), (x) => x.codePointAt(0));
}

/**
 * Formats a error message with context and a caret indicating the error position.
 * 
 * @param {string} input - The original input string.
 * @param {number} errorIndex - The UTF-16 index of the error in the input string.
 * @param {string} errorMessage - The error message to display.
 * @returns {string} The formatted error message.
 */
function formatError(input, errorIndex, errorMessage) {
  const segmenter = new Intl.Segmenter('en', { granularity: 'grapheme' });
  const graphemes = [...segmenter.segment(input)].map(s => s.segment);
  
  // Find which grapheme cluster the UTF-16 index falls into
  let graphemeIndex = 0;
  for (let utf16Pos = 0; utf16Pos < errorIndex && graphemeIndex < graphemes.length; graphemeIndex++) {
    utf16Pos += graphemes[graphemeIndex].length;
  }
  
  // Find context bounds, stopping at newlines
  const isNewline = (s) => s === '\n' || s === '\r';
  const contextSize = 15;
  
  let start = graphemeIndex;
  for (let i = 0; i < contextSize && start > 0 && !isNewline(graphemes[start - 1]); i++) {
    start--;
  }
  
  let end = graphemeIndex;
  for (let i = 0; i < contextSize && end < graphemes.length && !isNewline(graphemes[end]); i++) {
    end++;
  }
  
  // Check if there's more content on this line beyond our window
  const hasMoreBefore = start > 0 && !isNewline(graphemes[start - 1]);
  const hasMoreAfter = end < graphemes.length && !isNewline(graphemes[end]);
  
  const prefix = hasMoreBefore ? '...' : '';
  const suffix = hasMoreAfter ? '...' : '';
  const contextString = prefix + graphemes.slice(start, end).join('') + suffix;
  const caretLine = ' '.repeat(prefix.length + graphemeIndex - start) + '^';
  
  return [
    `Error: ${errorMessage}`,
    '|',
    `| ${contextString}`,
    `| ${caretLine}`,
    '|'
  ].join('\n');
}

document.addEventListener("DOMContentLoaded", () => {
  init().then(async () => {
    console.log("WASM module initialized");
    updateConfig(); // Initial config setup
    await loadFromUrl(); // Load content from URL hash if available
  });

  // Initialize cached values on page load
  initializeCachedValues();

  const inputField = document.getElementById("inputField");
  const outputField = document.getElementById("outputField");
  const outputCode = document.getElementById("outputCode");
  const configField = document.getElementById("configField");

  function updateOutput() {
    const input = inputField.value;
    try {
      const output = cachedConverter.convert_with_local_counter(
        input,
        isBlock(),
      );
      outputField.innerHTML = output;
      outputCode.textContent = output;
    } catch (error) {
      outputField.innerHTML = "";
      // outputCode.textContent = `Error at location ${error.location}: ${error.message}`;
      outputCode.textContent = formatError(input, error.location, error.message);
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
        updateConfig(); // Update config when radio button changes
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
    "Libertinus Math Regular": `font-feature-settings: "ss09";
    mtext {
        font-family: "Libertinus Serif", serif;
        code {
            font-family: "Libertinus Mono", monospace;
        }
        span.math-core-serif-font {
            font-family: "Libertinus Serif", serif;
        }
        span.math-core-sans-serif-font {
            font-family: "Libertinus Sans", sans-serif;
        }
    }`,
    "NewComputerModernMath Book": `
    mtext {
        font-family: "NewComputerModern Book", serif;
        code {
            font-family: "NewComputerModern Mono", monospace;
        }
        span.math-core-serif-font {
            font-family: "NewComputerModern Book", serif;
        }
        span.math-core-sans-serif-font {
            font-family: "NewComputerModern Sans", sans-serif;
        }
    }`,
    "Noto Sans Math Regular": `
    mtext {
        font-family: "Noto Sans", sans-serif;
        code {
            font-family: "Noto Sans Mono", monospace;
        }
        span.math-core-serif-font {
            font-family: "Noto Serif", serif;
        }
        span.math-core-sans-serif-font {
            font-family: "Noto Sans", sans-serif;
        }
    }`,
  };

  // Update the style rule when selection changes
  fontSelect.addEventListener("change", function () {
    const featureSettings = fontFeaturesMap[this.value]
      ? fontFeaturesMap[this.value]
      : "";
    styleElement.textContent = `math { font-family: "${this.value}", math; ${featureSettings} }`;
  });

  document
    .getElementById("generateBtn")
    .addEventListener("click", generateLink);
});
