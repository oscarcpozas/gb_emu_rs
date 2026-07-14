import init, { WasmEmu } from "./pkg/gb_wasm.js";

const WIDTH = 160;
const HEIGHT = 144;
const CORE_SAMPLE_RATE = 44100;
const TARGET_FRAME_MS = 1000 / 59.7275;
const MAX_FRAME_CATCH_UP = 3;
const AUDIO_BUFFER_SIZE = 2048;
const MAX_QUEUED_SECONDS = 0.5;

const dropZone = document.querySelector("#drop-zone");
const player = document.querySelector("#player");
const canvas = document.querySelector("#screen");
const status = document.querySelector("#status");
const batteryIndicator = document.querySelector("#battery-indicator");
const pickRom = document.querySelector("#pick-rom");
const romInput = document.querySelector("#rom-input");
const ctx = canvas.getContext("2d");
const image = ctx.createImageData(WIDTH, HEIGHT);

let emulator = null;
let running = false;
let lastEmulatedFrameTime = 0;
let frameTimeAccumulator = 0;
let audioContext = null;
let audioNode = null;
let audioQueue = [];
let queuedSamples = 0;

await init();

pickRom.addEventListener("click", () => {
  startAudio().catch(() => {});
  romInput.click();
});
romInput.addEventListener("change", async () => {
  const file = romInput.files?.[0];
  if (file) {
    await loadRom(file);
  }
});

for (const eventName of ["dragenter", "dragover", "dragleave", "drop"]) {
  document.addEventListener(eventName, handleDocumentDragEvent, { capture: true });
}

window.addEventListener("keydown", (event) => setKey(event, true));
window.addEventListener("keyup", (event) => setKey(event, false));
window.addEventListener("pointerdown", unlockAudio, { capture: true });
window.addEventListener("keydown", unlockAudio, { capture: true });

async function loadRom(file) {
  if (!file.name.toLowerCase().match(/\.(gb|gbc)$/)) {
    status.textContent = "Please choose a .gb or .gbc file.";
    return;
  }

  const bytes = new Uint8Array(await file.arrayBuffer());
  emulator = new WasmEmu(bytes);
  batteryIndicator.classList.add("on");
  dropZone.hidden = true;
  player.hidden = false;
  status.textContent = "Controls: arrows = D-pad, Z = A, X = B, Enter = Start, Shift = Select.";

  if (!running) {
    running = true;
    lastEmulatedFrameTime = 0;
    frameTimeAccumulator = 0;
    requestAnimationFrame(frame);
  }

  startAudio().catch(() => {
    status.textContent = "Game loaded. Click the page if your browser blocked audio.";
  });
}

async function handleDocumentDragEvent(event) {
  event.preventDefault();
  event.stopPropagation();

  if (event.dataTransfer) {
    event.dataTransfer.dropEffect = "copy";
  }

  if (event.type === "dragenter" || event.type === "dragover") {
    if (!emulator) {
      dropZone.classList.add("dragging");
    }
    return;
  }

  if (event.type === "dragleave") {
    if (event.relatedTarget === null) {
      dropZone.classList.remove("dragging");
    }
    return;
  }

  dropZone.classList.remove("dragging");

  const file = getDroppedFile(event.dataTransfer);
  if (!file) {
    status.textContent = "No ROM file detected in the drop.";
    return;
  }

  startAudio().catch(() => {});
  await loadRom(file);
}

function getDroppedFile(dataTransfer) {
  if (!dataTransfer) {
    return null;
  }

  for (const item of dataTransfer.items ?? []) {
    if (item.kind === "file") {
      const file = item.getAsFile();
      if (file) {
        return file;
      }
    }
  }

  return dataTransfer.files?.[0] ?? null;
}

function frame(timestamp) {
  if (emulator) {
    const framesToRun = consumeDueFrames(timestamp);
    for (let i = 0; i < framesToRun; i += 1) {
      emulator.step_frame();
      enqueueAudio(emulator.drain_audio_samples());
    }

    if (framesToRun > 0) {
      image.data.set(emulator.frame_rgba());
      ctx.putImageData(image, 0, 0);
    }
  }

  requestAnimationFrame(frame);
}

function consumeDueFrames(timestamp) {
  if (lastEmulatedFrameTime === 0) {
    lastEmulatedFrameTime = timestamp;
    return 1;
  }

  frameTimeAccumulator += timestamp - lastEmulatedFrameTime;
  lastEmulatedFrameTime = timestamp;

  let frames = 0;
  while (frameTimeAccumulator >= TARGET_FRAME_MS && frames < MAX_FRAME_CATCH_UP) {
    frameTimeAccumulator -= TARGET_FRAME_MS;
    frames += 1;
  }

  if (frames === MAX_FRAME_CATCH_UP) {
    frameTimeAccumulator = Math.min(frameTimeAccumulator, TARGET_FRAME_MS);
  }

  return frames;
}

function unlockAudio() {
  if (emulator && audioContext?.state !== "running") {
    startAudio().catch(() => {});
  }
}

async function startAudio() {
  if (!audioContext) {
    const AudioContext = window.AudioContext || window.webkitAudioContext;
    if (!AudioContext) {
      status.textContent = "Audio is not supported by this browser.";
      return;
    }

    try {
      audioContext = new AudioContext({ sampleRate: CORE_SAMPLE_RATE });
    } catch {
      audioContext = new AudioContext();
    }
    audioNode = audioContext.createScriptProcessor(AUDIO_BUFFER_SIZE, 0, 1);
    audioNode.onaudioprocess = fillAudioBuffer;
    audioNode.connect(audioContext.destination);
  }

  if (audioContext.state !== "running") {
    await audioContext.resume();
  }
}

function enqueueAudio(samples) {
  if (!audioContext || samples.length === 0) {
    return;
  }

  const outputSamples = audioContext.sampleRate === CORE_SAMPLE_RATE
    ? samples
    : resample(samples, CORE_SAMPLE_RATE, audioContext.sampleRate);

  const maxQueuedSamples = Math.floor(audioContext.sampleRate * MAX_QUEUED_SECONDS);
  if (queuedSamples > maxQueuedSamples) {
    return;
  }

  audioQueue.push(outputSamples);
  queuedSamples += outputSamples.length;
}

function fillAudioBuffer(event) {
  const output = event.outputBuffer.getChannelData(0);
  let offset = 0;

  while (offset < output.length && audioQueue.length > 0) {
    const chunk = audioQueue[0];
    const available = Math.min(chunk.length, output.length - offset);
    output.set(chunk.subarray(0, available), offset);
    offset += available;
    queuedSamples -= available;

    if (available === chunk.length) {
      audioQueue.shift();
    } else {
      audioQueue[0] = chunk.subarray(available);
    }
  }

  output.fill(0, offset);
}

function resample(samples, fromRate, toRate) {
  const ratio = fromRate / toRate;
  const length = Math.max(1, Math.floor(samples.length / ratio));
  const output = new Float32Array(length);

  for (let i = 0; i < length; i += 1) {
    const sourceIndex = i * ratio;
    const index = Math.floor(sourceIndex);
    const nextIndex = Math.min(index + 1, samples.length - 1);
    const t = sourceIndex - index;
    output[i] = samples[index] * (1 - t) + samples[nextIndex] * t;
  }

  return output;
}

function setKey(event, pressed) {
  if (!emulator) {
    return;
  }

  const key = mapKey(event.code);
  if (!key) {
    return;
  }

  event.preventDefault();
  emulator.set_key(key, pressed);
}

function mapKey(code) {
  switch (code) {
    case "ArrowRight":
      return "right";
    case "ArrowLeft":
      return "left";
    case "ArrowUp":
      return "up";
    case "ArrowDown":
      return "down";
    case "KeyZ":
      return "a";
    case "KeyX":
      return "b";
    case "ShiftRight":
    case "ShiftLeft":
      return "select";
    case "Enter":
      return "start";
    default:
      return null;
  }
}
