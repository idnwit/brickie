// The Monaco glue for the code viewer app (window.cv). Monaco loads through its AMD loader
// from static/vs; calls made before it is ready wait for it. Positions crossing this boundary
// are 1-based lines and 1-based UTF-8 byte columns (the port's); Monaco's are UTF-16.
(function () {
  "use strict";
  const enc = new TextEncoder();
  let editor = null;
  let resolveReady;
  const ready = new Promise((r) => (resolveReady = r));
  const models = new Map();
  // Bands (nested gutter bars): per path, the shown flag, and the editor's decorations.
  const bands = new Map();
  let bandsOn = true;
  let bandDecos = null;
  let selDecos = null;
  let tip = null;
  const COL = 6; // px per band column

  require.config({ paths: { vs: "static/vs" } });
  const base = new URL("static/", location.href).href;
  window.MonacoEnvironment = {
    getWorkerUrl: () => "data:text/javascript;charset=utf-8," + encodeURIComponent(
      `self.MonacoEnvironment={baseUrl:${JSON.stringify(base)}};` +
      `importScripts(${JSON.stringify(base + "vs/base/worker/workerMain.js")});`),
  };

  function byteToUtf16(text, byteCol) {
    let bytes = 0;
    let i = 0;
    while (i < text.length && bytes < byteCol - 1) {
      const cp = text.codePointAt(i);
      bytes += enc.encode(String.fromCodePoint(cp)).length;
      i += cp > 0xffff ? 2 : 1;
    }
    return i + 1;
  }
  function utf16ToByte(text, col) {
    return enc.encode(text.slice(0, col - 1)).length + 1;
  }
  function lineText(model, line) {
    return line >= 1 && line <= model.getLineCount() ? model.getLineContent(line) : "";
  }

  function at(ed, cb) {
    const p = ed.getPosition();
    const m = ed.getModel();
    if (p && m) cb(p.lineNumber, utf16ToByte(lineText(m, p.lineNumber), p.column));
  }

  // The bands of the shown model as gutter decorations; the gutter widens to fit the depth.
  function applyBands() {
    if (!editor || !bandDecos) return;
    const model = editor.getModel();
    const path = [...models].find(([, v]) => v === model)?.[0];
    const list = (bandsOn && path && bands.get(path)) || [];
    const depth = list.reduce((d, b) => Math.max(d, b.depth), -1);
    editor.updateOptions({ lineDecorationsWidth: list.length ? (depth + 1) * COL + 6 : 10 });
    bandDecos.set(list.map((b) => ({
      range: { startLineNumber: b.start_line, startColumn: 1, endLineNumber: b.end_line, endColumn: 1 },
      options: { linesDecorationsClassName:
        `cvb cvb-d${Math.min(b.depth, 15)} cvb-h${Math.round(Math.min(1, Math.max(0, b.heat)) * 9)}` },
    })));
    if (!list.length) selDecos.clear();
  }

  // The band under the mouse in the gutter: its depth column, else the deepest at the line.
  function bandAt(e) {
    const m = window.monaco;
    if (!bandsOn || e.target.type !== m.editor.MouseTargetType.GUTTER_LINE_DECORATIONS) return null;
    const line = e.target.position && e.target.position.lineNumber;
    const path = [...models].find(([, v]) => v === editor.getModel())?.[0];
    const here = (bands.get(path) || []).filter((b) => b.start_line <= line && line <= b.end_line);
    if (!here.length) return null;
    const left = editor.getLayoutInfo().decorationsLeft;
    const col = Math.floor((e.target.detail.offsetX - left) / COL);
    return here.find((b) => b.depth === col) || here.reduce((a, b) => (b.depth > a.depth ? b : a));
  }

  function showTip(b, ev) {
    if (!tip) {
      tip = document.createElement("div");
      tip.className = "cvb-tip";
      document.body.appendChild(tip);
    }
    if (!b) { tip.style.display = "none"; return; }
    tip.textContent = "";
    const h = document.createElement("b");
    h.textContent = b.label;
    const d = document.createElement("div");
    d.textContent = `lines ${b.start_line}-${b.end_line}` + (b.detail ? "\n" + b.detail : "");
    tip.append(h, d);
    tip.style.display = "block";
    tip.style.left = ev.event.browserEvent.clientX + 14 + "px";
    tip.style.top = ev.event.browserEvent.clientY + 10 + "px";
  }

  function selectBand(b) {
    const range = { startLineNumber: b.start_line, startColumn: 1, endLineNumber: b.end_line, endColumn: 1 };
    selDecos.set([{ range, options: { isWholeLine: true, className: "cvb-sel" } }]);
    editor.revealRangeInCenterIfOutsideViewport(range);
  }

  window.cv = {
    init(el, dark, onDef, onRefs, onQuick) {
      require(["vs/editor/editor.main"], function () {
        const m = window.monaco;
        editor = m.editor.create(el, {
          readOnly: true,
          automaticLayout: true,
          theme: dark ? "vs-dark" : "vs",
          minimap: { enabled: true },
          fontSize: 13,
          scrollBeyondLastLine: false,
          glyphMargin: true,
        });
        editor.addAction({ id: "cv.def", label: "Go to Definition", keybindings: [m.KeyCode.F12],
          contextMenuGroupId: "navigation", run: (ed) => at(ed, onDef) });
        editor.addAction({ id: "cv.refs", label: "Find References",
          keybindings: [m.KeyMod.Shift | m.KeyCode.F12], contextMenuGroupId: "navigation",
          run: (ed) => at(ed, onRefs) });
        editor.addAction({ id: "cv.quick", label: "Go to File",
          keybindings: [m.KeyMod.CtrlCmd | m.KeyCode.KeyP], run: () => onQuick() });
        bandDecos = editor.createDecorationsCollection();
        selDecos = editor.createDecorationsCollection();
        editor.onMouseMove((e) => showTip(bandAt(e), e));
        editor.onMouseLeave(() => showTip(null));
        editor.onMouseDown((e) => { const b = bandAt(e); if (b) selectBand(b); });
        editor.onDidChangeModel(() => { selDecos.clear(); applyBands(); });
        resolveReady(m);
      });
    },
    has(path) { return models.has(path); },
    load(path, text, lang) {
      ready.then((m) => {
        if (models.has(path)) return;
        const uri = m.Uri.from({ scheme: "file", path: "/" + path });
        const old = m.editor.getModel(uri);
        if (old) old.dispose();
        models.set(path, m.editor.createModel(text, lang, uri));
      });
    },
    show(path, line, col) {
      ready.then(() => {
        const model = models.get(path);
        if (!model) return;
        if (editor.getModel() !== model) editor.setModel(model);
        if (line > 0) {
          const c = col > 0 ? byteToUtf16(lineText(model, line), col) : 1;
          editor.setSelection({ startLineNumber: line, startColumn: c, endLineNumber: line, endColumn: c });
          editor.revealLineInCenter(line);
        }
        editor.focus();
      });
    },
    dispose(path) {
      const model = models.get(path);
      models.delete(path);
      bands.delete(path);
      if (model) ready.then(() => model.dispose());
    },
    setMarks(path, json) {
      ready.then((m) => {
        const model = models.get(path);
        if (!model) return;
        const sev = { error: m.MarkerSeverity.Error, warn: m.MarkerSeverity.Warning, info: m.MarkerSeverity.Info };
        const marks = JSON.parse(json).map((k) => {
          const s = k.span.start, e = k.span.end;
          return {
            severity: sev[k.level] || m.MarkerSeverity.Info,
            message: k.message,
            startLineNumber: s.line,
            startColumn: byteToUtf16(lineText(model, s.line), Math.max(1, s.col)),
            endLineNumber: e.line,
            endColumn: e.col > 0 ? byteToUtf16(lineText(model, e.line), e.col) : model.getLineMaxColumn(e.line),
          };
        });
        m.editor.setModelMarkers(model, "host", marks);
      });
    },
    setBands(path, json) {
      bands.set(path, JSON.parse(json));
      ready.then(() => applyBands());
    },
    showBands(on) {
      bandsOn = on;
      ready.then(() => { if (!on) showTip(null); applyBands(); });
    },
    theme(dark) {
      ready.then((m) => m.editor.setTheme(dark ? "vs-dark" : "vs"));
    },
  };
})();
