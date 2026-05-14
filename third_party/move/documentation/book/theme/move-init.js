if (typeof hljs !== "undefined" && typeof hljsDefineMove === "function") {
    hljs.registerLanguage("move", hljsDefineMove);

    const moveBlocks = document.querySelectorAll(
        "pre code.language-move, pre code.lang-move, pre code.language-aptos-move, pre code.language-move-on-aptos, pre code.language-move-lang"
    );

    moveBlocks.forEach((block) => {
        if (block.dataset && block.dataset.highlighted) {
            delete block.dataset.highlighted;
        }
        hljs.highlightElement(block);
    });
}
