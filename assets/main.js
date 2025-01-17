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
  diffusionA: 1.0,
  diffusionB: 0.5,
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
  updateCallbacks[key](value);
  outputs[key].innerHTML = value.toString();
  inputs[key].value = value;
}

// initialise UI
function initUI(updater) {
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
      setValue(key, event.target.value);
    });
  }
}

let updater = null;

init()
  .then(() => {
    // TODO: do not hardcode window size (after supporting resize)
    const config = Config.with_size(512, 512);
    const app = App.new(config);
    app.mountCanvas();
    updater = app.updater();
    return app.run();
  })
  .then(() => initUI(updater));
