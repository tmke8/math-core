import { convert } from './pkg/latex2mmlc_wasm.js';

document.addEventListener('DOMContentLoaded', () => {
    const inputField = document.getElementById('inputField');
    const outputField = document.getElementById('outputField');

    inputField.addEventListener('input', () => {
        try {
            const input = inputField.value;
            const output = convert(input);
            outputField.innerHTML = output;
        } catch (error) {
            outputField.textContent = `Error: ${error.message}`;
        }
    });
});

