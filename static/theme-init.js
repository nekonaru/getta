// Restore the saved theme before first paint to avoid a flash.
// External file so the strict CSP (script-src 'self') can stay intact.
(function () {
	try {
		var t = localStorage.getItem("getta.theme");
		if (t === "light" || t === "dark" || t === "system") {
			document.documentElement.dataset.theme = t;
		}
	} catch (e) {
		/* storage unavailable: keep default */
	}
})();
