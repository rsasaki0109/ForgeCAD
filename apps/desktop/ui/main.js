const { invoke } = window.__TAURI__.core;
const { open, save } = window.__TAURI__.dialog;

const preview = document.getElementById("preview");
const status = document.getElementById("status");
const docInfo = document.getElementById("doc-info");
const previewInfo = document.getElementById("preview-info");
const selectionInfo = document.getElementById("selection-info");
const parametersPanel = document.getElementById("parameters");
const templateSelect = document.getElementById("template-select");
const openBtn = document.getElementById("open-btn");
const refreshBtn = document.getElementById("refresh-btn");
const viewportBtn = document.getElementById("viewport-btn");
const createBtn = document.getElementById("create-btn");

let currentPath = null;
let parameterRows = [];

const PREVIEW_WIDTH = 960;
const PREVIEW_HEIGHT = 540;

function setStatus(message) {
  status.textContent = message;
}

function renderInfo(container, entries) {
  container.replaceChildren();
  for (const [label, value] of entries) {
    const dt = document.createElement("dt");
    dt.textContent = label;
    const dd = document.createElement("dd");
    dd.textContent = String(value);
    container.append(dt, dd);
  }
}

function formatVec3(values) {
  return values.map((v) => v.toFixed(4)).join(", ");
}

function previewImageCoords(event) {
  const rect = preview.getBoundingClientRect();
  const naturalW = preview.naturalWidth || PREVIEW_WIDTH;
  const naturalH = preview.naturalHeight || PREVIEW_HEIGHT;
  const scale = Math.min(rect.width / naturalW, rect.height / naturalH);
  const renderW = naturalW * scale;
  const renderH = naturalH * scale;
  const offsetX = (rect.width - renderW) / 2;
  const offsetY = (rect.height - renderH) / 2;
  const x = ((event.clientX - rect.left - offsetX) / renderW) * naturalW;
  const y = ((event.clientY - rect.top - offsetY) / renderH) * naturalH;
  if (x < 0 || y < 0 || x > naturalW || y > naturalH) {
    return null;
  }
  return { x, y };
}

function renderSelection(summary) {
  const entries = [
    ["Pixel", `${summary.x.toFixed(1)}, ${summary.y.toFixed(1)}`],
    ["Kind", summary.selection.kind ?? "none"],
  ];

  if (summary.selection.kind === "sketch_line") {
    const line = summary.selection;
    entries.push(
      ["Line index", line.line_index],
      ["Sketch", line.sketch_id ?? "—"],
      ["Entity", line.entity_id ?? "—"],
      ["Entity kind", line.entity_kind ?? "—"],
      ["Construction", line.construction],
      ["Start (m)", formatVec3(line.start_m)],
      ["End (m)", formatVec3(line.end_m)],
    );
  } else if (summary.selection.kind === "solid_triangle") {
    const solid = summary.selection;
    entries.push(
      ["Triangle", solid.triangle_index],
      ["Face group", solid.face_group_index ?? "—"],
      ["Face role", solid.face_role ?? "—"],
      ["Kernel face", solid.kernel_face_id ?? "—"],
      ["Feature", solid.inferred_feature_id ?? "—"],
      ["Topo ref", solid.inferred_topo_ref_id ?? "—"],
    );
    if (solid.face_centroid_m) {
      entries.push(["Centroid (m)", formatVec3(solid.face_centroid_m)]);
    }
    if (solid.face_normal_m) {
      entries.push(["Normal (m)", formatVec3(solid.face_normal_m)]);
    }
  }

  renderInfo(selectionInfo, entries);
}

function clearSelection() {
  renderInfo(selectionInfo, [["Kind", "none"]]);
}

async function pickAtPreview(event) {
  if (!currentPath) {
    return;
  }
  const coords = previewImageCoords(event);
  if (!coords) {
    return;
  }

  setStatus(`Picking at ${coords.x.toFixed(0)}, ${coords.y.toFixed(0)}…`);
  try {
    const summary = await invoke("pick_document_cmd", {
      path: currentPath,
      x: coords.x,
      y: coords.y,
    });
    renderSelection(summary);
    if (summary.selection.kind === "none") {
      setStatus("No geometry at click point.");
    } else {
      setStatus(`Selected ${summary.selection.kind.replaceAll("_", " ")}`);
    }
  } catch (error) {
    setStatus(`Error: ${error}`);
  }
}

function formatValueMm(valueMm) {
  if (valueMm == null) {
    return "—";
  }
  return `${valueMm.toFixed(2)} mm`;
}

async function applyParameter(row, input) {
  const nextExpr = input.value.trim();
  if (!nextExpr || nextExpr === row.expr) {
    input.value = row.expr;
    return;
  }

  setStatus(`Updating ${row.name}…`);
  try {
    await invoke("set_document_parameter_cmd", {
      path: currentPath,
      id: row.id,
      expr: nextExpr,
    });
    await loadDocument(currentPath);
    setStatus(`Updated ${row.name}`);
  } catch (error) {
    input.value = row.expr;
    setStatus(`Error: ${error}`);
  }
}

function renderParameters(rows) {
  parameterRows = rows;
  parametersPanel.replaceChildren();

  if (!rows.length) {
    const empty = document.createElement("p");
    empty.className = "parameters-empty";
    empty.textContent = "No parameters.";
    parametersPanel.append(empty);
    return;
  }

  for (const row of rows) {
    const wrapper = document.createElement("div");
    wrapper.className = "param-row";

    const label = document.createElement("label");
    label.textContent = row.name;
    label.htmlFor = `param-${row.id}`;

    const input = document.createElement("input");
    input.id = `param-${row.id}`;
    input.type = "text";
    input.value = row.expr;
    input.spellcheck = false;
    input.addEventListener("keydown", (event) => {
      if (event.key === "Enter") {
        event.preventDefault();
        input.blur();
      }
    });
    input.addEventListener("blur", () => {
      applyParameter(row, input).catch((error) => setStatus(`Error: ${error}`));
    });

    const value = document.createElement("span");
    value.className = "param-value";
    value.textContent = formatValueMm(row.value_mm);

    wrapper.append(label, input, value);
    parametersPanel.append(wrapper);
  }
}

async function loadParameters() {
  if (!currentPath) {
    renderParameters([]);
    return;
  }
  const rows = await invoke("list_document_parameters_cmd", { path: currentPath });
  renderParameters(rows);
}

async function loadTemplates() {
  const templates = await invoke("list_templates");
  templateSelect.replaceChildren();
  for (const template of templates) {
    const option = document.createElement("option");
    option.value = template.id;
    option.textContent = template.label;
    templateSelect.append(option);
  }
}

async function loadDocument(path, options = {}) {
  if (!path) {
    setStatus("No document selected.");
    return;
  }

  currentPath = path;
  setStatus(`Regenerating ${path}…`);

  const requests = [
    invoke("inspect_document_cmd", { path }),
    invoke("preview_document_cmd", { path }),
  ];
  if (!options.skipParameters) {
    requests.push(invoke("list_document_parameters_cmd", { path }));
  }

  const results = await Promise.all(requests);
  const inspect = results[0];
  const previewData = results[1];

  preview.src = `data:image/png;base64,${previewData.png_base64}`;
  preview.alt = previewData.name;

  renderInfo(docInfo, [
    ["Name", inspect.name],
    ["ID", inspect.id],
    ["Path", path],
    ["Sketches", inspect.sketches],
    ["Features", inspect.features],
    ["Parameters", inspect.parameters],
    ["Topo refs", inspect.semantic_refs],
  ]);

  renderInfo(previewInfo, [
    ["Triangles", previewData.triangles],
    ["Vertices", previewData.vertices],
    [
      "Bounds min (m)",
      previewData.bounds_min_m.map((v) => v.toFixed(4)).join(", "),
    ],
    [
      "Bounds max (m)",
      previewData.bounds_max_m.map((v) => v.toFixed(4)).join(", "),
    ],
  ]);

  if (!options.skipParameters && results[2]) {
    renderParameters(results[2]);
  }

  clearSelection();
  setStatus(`Loaded ${previewData.name}`);
}

async function openDocument() {
  const selected = await open({
    directory: true,
    multiple: false,
    title: "Open .ocad.d directory",
  });
  if (selected) {
    await loadDocument(selected);
  }
}

async function openViewport() {
  if (!currentPath) {
    setStatus("Open a document first.");
    return;
  }
  setStatus("Opening 3D viewport…");
  await invoke("open_viewport_cmd", { path: currentPath });
  setStatus("3D viewport running in a separate window.");
}

async function createSample() {
  const selected = await save({
    title: "Create sample document",
    defaultPath: "sample.ocad.d",
  });
  if (!selected) {
    return;
  }
  const templateId = templateSelect.value;
  await invoke("create_template_document", {
    path: selected,
    templateId,
  });
  await loadDocument(selected);
}

async function boot() {
  try {
    await loadTemplates();
    const defaultPath = await invoke("default_example_path");
    if (defaultPath) {
      await loadDocument(defaultPath);
    } else {
      setStatus("Open a .ocad.d directory to preview.");
      renderParameters([]);
      clearSelection();
    }
  } catch (error) {
    setStatus(`Error: ${error}`);
  }
}

openBtn.addEventListener("click", () => {
  openDocument().catch((error) => setStatus(`Error: ${error}`));
});

preview.addEventListener("click", (event) => {
  pickAtPreview(event).catch((error) => setStatus(`Error: ${error}`));
});

refreshBtn.addEventListener("click", () => {
  loadDocument(currentPath).catch((error) => setStatus(`Error: ${error}`));
});

viewportBtn.addEventListener("click", () => {
  openViewport().catch((error) => setStatus(`Error: ${error}`));
});

createBtn.addEventListener("click", () => {
  createSample().catch((error) => setStatus(`Error: ${error}`));
});

boot();
