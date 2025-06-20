const copyButtonLabel = "Copy code";
const copyButtonLabelCopied = "Code copied!";

function docReady(fn) {
  // see if DOM is already available
  if (
    document.readyState === "complete" ||
    document.readyState === "interactive"
  ) {
    // call on next available tick
    setTimeout(fn, 1);
  } else {
    document.addEventListener("DOMContentLoaded", fn);
  }
}

docReady(function () {
  // DOM is loaded and ready for manipulation here
  // use a class selector if available
  let blocks = document.querySelectorAll(".copyable");

  blocks.forEach((block) => {
    // only add button if browser supports Clipboard API
    if (navigator.clipboard) {
      let button = document.createElement("button");

      button.innerText = copyButtonLabel;
      block.appendChild(button);

      let code = block.querySelector("code");

      button.addEventListener("click", async () => {
        await copyCode(code, button);
      });
    }
  });

  const linkButton = document.getElementById('copyBtn')
  const linkField = document.getElementById('generatedLink');
  linkButton.addEventListener('click', async () => {
    await copyCode(linkField, linkButton);
  });

  async function copyCode(code, button) {
    let text = code.innerText;

    await navigator.clipboard.writeText(text);

    // visual feedback that task is completed
    button.innerText = copyButtonLabelCopied;

    setTimeout(() => {
      button.innerText = copyButtonLabel;
    }, 700);
  }
});
