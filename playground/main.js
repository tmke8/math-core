import { convert } from './pkg/latex2mmlc_wasm.js';

function isBlock() {
    // Query the fieldset for the checked radio button
    var selectedRadio = document.querySelector('#displaystyle input[type="radio"]:checked');
    
    // Check the value and return true if it's 'block', false if it's 'inline'
    return selectedRadio ? selectedRadio.value === 'block' : true;
}

document.addEventListener('DOMContentLoaded', () => {
    const inputField = document.getElementById('inputField');
    const outputField = document.getElementById('outputField');
    const outputCode = document.getElementById('outputCode');

    function updateOutput() {
        try {
            const input = inputField.value;
            const output = convert(input, isBlock());
            outputField.innerHTML = output;
            outputCode.textContent = output;
        } catch (error) {
            outputField.innerHTML = "";
            outputCode.textContent = `Error: ${error}`;
        }
    }

    inputField.addEventListener('input', () => {
        updateOutput();
    });

    document.querySelectorAll('#displaystyle input[type="radio"]').forEach((radio) => {
        radio.addEventListener('change', () => {
            updateOutput();
        });
    });
});

