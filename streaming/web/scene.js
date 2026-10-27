// K7 spike scene: the Example L bracket (100 x 60 x 80 mm, as in examples/assets/example-bracket),
// studio lights, orbit controls, and a latency marker drawn inside the WebGL frame.
// The marker (64 px square, top-left) switches red <-> green on each "m" key, so a measured
// input-to-photon time includes a real WebGL frame. This page is a stand-in for the Studio canvas
// (UI track); it is labelled Example.
import * as THREE from "three";
import { OrbitControls } from "three/addons/controls/OrbitControls.js";

const renderer = new THREE.WebGLRenderer({ antialias: true });
renderer.setPixelRatio(1);
document.body.appendChild(renderer.domElement);

const scene = new THREE.Scene();
scene.background = new THREE.Color(0x3a3c40);
const camera = new THREE.PerspectiveCamera(40, 16 / 9, 0.01, 10);
camera.position.set(0.22, -0.28, 0.2);
camera.up.set(0, 0, 1);
const controls = new OrbitControls(camera, renderer.domElement);
controls.target.set(0.04, 0.03, 0.03);
controls.update();

scene.add(new THREE.HemisphereLight(0xffffff, 0x445566, 1.2));
const key = new THREE.DirectionalLight(0xffffff, 2.2);
key.position.set(1, -1.2, 1.4);
scene.add(key);
const steel = new THREE.MeshStandardMaterial({ color: 0x9ea1a6, metalness: 0.6, roughness: 0.35 });
const base = new THREE.Mesh(new THREE.BoxGeometry(0.1, 0.06, 0.006), steel);
base.position.set(0.05, 0.03, 0.003);
const upright = new THREE.Mesh(new THREE.BoxGeometry(0.006, 0.06, 0.074), steel);
upright.position.set(0.003, 0.03, 0.006 + 0.037);
scene.add(base, upright, new THREE.GridHelper(0.3, 30, 0x666666, 0x4a4a4a).rotateX(Math.PI / 2));

// Latency marker state: null (none yet), then red, green, red, ...
const MARKER = [new THREE.Color(1, 0, 0), new THREE.Color(0, 1, 0)];
let markerIndex = -1;
window.__markerIndex = () => markerIndex; // read by the server, to tell input problems from drawing problems
addEventListener("keydown", (e) => { if (e.key === "m") markerIndex = (markerIndex + 1) % 2; });

// Size from the window on every frame: at start-up a kiosk window can still report 0 x 0.
function fit() {
  const w = innerWidth, h = innerHeight;
  if (w > 0 && h > 0 && (renderer.domElement.width !== w || renderer.domElement.height !== h)) {
    renderer.setSize(w, h);
    camera.aspect = w / h;
    camera.updateProjectionMatrix();
  }
}

const clear = new THREE.Color();
function frame() {
  fit();
  controls.update();
  renderer.setScissorTest(false);
  renderer.render(scene, camera);
  if (markerIndex >= 0) {
    renderer.getClearColor(clear);
    renderer.setScissorTest(true);
    renderer.setScissor(16, renderer.domElement.height - 80, 64, 64); // WebGL origin is bottom-left: top-left square
    renderer.setClearColor(MARKER[markerIndex]);
    renderer.clear(true, false, false);
    // Read back what was drawn at the marker's centre, so the server can tell drawing from capture problems.
    const px = new Uint8Array(4);
    renderer.getContext().readPixels(48, renderer.domElement.height - 48, 1, 1, renderer.getContext().RGBA, renderer.getContext().UNSIGNED_BYTE, px);
    window.__markerPixel = [px[0], px[1], px[2], px[3]];
    renderer.setScissorTest(false);
    renderer.setClearColor(clear);
  }
  requestAnimationFrame(frame);
}
requestAnimationFrame(frame);

const gl = renderer.getContext();
const dbg = gl.getExtension("WEBGL_debug_renderer_info");
window.rendererInfo = dbg ? gl.getParameter(dbg.UNMASKED_RENDERER_WEBGL) : gl.getParameter(gl.RENDERER);
// Ready only once a real frame size exists.
(function waitForSize() {
  const fitted = innerWidth > 0 && renderer.domElement.width === innerWidth && renderer.domElement.height === innerHeight;
  if (fitted) window.sceneReady = true; else requestAnimationFrame(waitForSize);
})();
