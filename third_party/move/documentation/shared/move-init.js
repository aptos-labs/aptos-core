if (typeof hljs !== "undefined" && typeof hljsDefineMove === "function") {
    hljs.registerLanguage("move", hljsDefineMove);

    const moveBlocks = document.querySelectorAll(
        "pre code.language-move, pre code.lang-move, pre code.language-aptos-move, pre code.language-move-on-aptos, pre code.language-move-lang"
    );

    // hljs 10.7+/11 expose highlightElement; mdBook still ships hljs 10.1.1
    // which only has the (now-deprecated) highlightBlock. Support both.
    const highlightFn = hljs.highlightElement || hljs.highlightBlock;

    moveBlocks.forEach((block) => {
        if (block.dataset && block.dataset.highlighted) {
            delete block.dataset.highlighted;
        }
        highlightFn.call(hljs, block);
    });
}
