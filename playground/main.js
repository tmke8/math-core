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

async function generateLink() {
  const inputField = document.getElementById('inputField');
  const content = inputField.value;
  
  if (content.trim() === '') {
    alert('Please enter some text first!');
    return;
  }

  // Compress the content
  const compressedContent = await compressText(content, 'gzip')
  
  // Encode content to base64
  const encodedContent = compressedContent.toBase64({ alphabet: "base64url" });
  
  // Generate the URL
  const currentUrl = window.location.origin + window.location.pathname;
  const generatedUrl = `${currentUrl}#input:${encodedContent}`;
  
  // Display the link
  document.getElementById('generatedLink').innerText = generatedUrl;
  document.getElementById('linkContainer').classList.remove('hidden');
}

// Function to load content from URL hash
async function loadFromUrl() {
  const hash = window.location.hash;
  
  if (hash.startsWith('#input:')) {
    const encodedContent = hash.substring(7); // Remove '#input:' prefix
    
    try {
      const compressedContent = Uint8Array.fromBase64(encodedContent, { alphabet: "base64url" });
      const decodedContent = await decompressText(compressedContent);
      const inputField = document.getElementById('inputField');
      inputField.value = decodedContent;
      // Trigger input event to update output
      inputField.dispatchEvent(new Event('input', { bubbles: true }));

      // Clear the hash from URL without page reload
      history.replaceState(null, null, window.location.pathname + window.location.search);
    } catch (err) {
      console.error('Failed to decode content from URL:', err);
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
  const format = 'gzip';
  // Validate input
  if (typeof text !== 'string') {
    throw new TypeError('Input must be a string');
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
  const format = 'gzip';
  // Validate input
  if (!(compressedData instanceof Uint8Array)) {
    throw new TypeError('Input must be a Uint8Array');
  }

  // Create a stream from compressed data
  const blob = new Blob([compressedData]);
  const stream = blob.stream();
  
  // Decompress the stream
  const decompressedStream = stream.pipeThrough(new DecompressionStream(format));
  
  // Read and decode the result
  const response = new Response(decompressedStream);
  const arrayBuffer = await response.arrayBuffer();
  const decoder = new TextDecoder();
  return decoder.decode(arrayBuffer);
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

  document.getElementById('generateBtn').addEventListener('click', generateLink);
});
