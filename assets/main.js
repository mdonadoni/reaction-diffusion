import init, { Config, App } from "./reaction-diffusion.js";

const inputs = {
  diffusionA: document.getElementById("diffusion-a"),
  diffusionB: document.getElementById("diffusion-b"),
  feed: document.getElementById("feed"),
  kill: document.getElementById("kill"),
  stepsPerFrame: document.getElementById("steps-per-frame"),
  timestep: document.getElementById("timestep"),
};

const outputs = {
  diffusionA: document.getElementById("diffusion-a-value"),
  diffusionB: document.getElementById("diffusion-b-value"),
  feed: document.getElementById("feed-value"),
  kill: document.getElementById("kill-value"),
  stepsPerFrame: document.getElementById("steps-per-frame-value"),
  timestep: document.getElementById("timestep-value"),
};

const state = {
  diffusionA: 0.5,
  diffusionB: 0.25,
  feed: 0.03,
  kill: 0.09,
  stepsPerFrame: 20,
  timestep: 1.0,
};

let updateCallbacks = null;

const configKeys = [
  "diffusionA",
  "diffusionB",
  "feed",
  "kill",
  "stepsPerFrame",
  "timestep",
];

function setValue(key, value) {
  console.log(`Setting value ${key} to ${value}`);
  state[key] = value;
  inputs[key].value = value;
  updateCallbacks[key](value);
  // show rounded value in UI
  outputs[key].innerHTML = parseFloat(value.toFixed(4)).toString();
}

// initialise UI
function initUI(updater, canvas) {
  // mount canvas
  document.getElementById("canvas-container").appendChild(canvas);

  // set event listeners
  updateCallbacks = {
    diffusionA: updater.setDiffusionA.bind(updater),
    diffusionB: updater.setDiffusionB.bind(updater),
    feed: updater.setFeed.bind(updater),
    kill: updater.setKill.bind(updater),
    stepsPerFrame: updater.setStepsPerFrame.bind(updater),
    timestep: updater.setTimestep.bind(updater),
  };

  for (const key of configKeys) {
    // set initial value
    setValue(key, state[key]);
    inputs[key].addEventListener("input", (event) => {
      setValue(key, event.target.valueAsNumber);
    });
  }

  const resetButton = document.getElementById("reset");
  resetButton.addEventListener("click", updater.reset.bind(updater));
  const startButton = document.getElementById("start");
  startButton.addEventListener("click", updater.start.bind(updater));
  const pauseButton = document.getElementById("pause");
  pauseButton.addEventListener("click", updater.pause.bind(updater));
}

function isWebGPUAvailable() {
  if (!navigator.gpu) {
    return Promise.resolve(false);
  }

  return navigator.gpu
    .requestAdapter()
    .then((adapter) => !!adapter)
    .catch(() => false);
}

function showWebGPUError() {
  console.log("No WebGPU support!");
  const error = document.getElementById("webgpu-error");
  error.style.display = "block";
}

init()
  .then(isWebGPUAvailable)
  .then((webGPUAvailable) => {
    if (!webGPUAvailable) {
      showWebGPUError();
      return;
    }

    // TODO: do not hardcode window size (after supporting resize)
    const config = Config.with_size(512, 512);
    const app = App.new(config);
    const canvas = app.canvas();
    const updater = app.updater();
    return app.run().then(() => initUI(updater, canvas));
  });
