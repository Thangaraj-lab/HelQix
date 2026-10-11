/**
 * HELQIX Visual Runtime — SceneManager
 * Layer 14: 14-visual-runtime
 *
 * Orchestrates WebGL canvas lifecycle, multi-camera projection, resize events, and render ticks.
 */

import * as THREE from 'three';
import { EngineeringScene, SceneConfig } from './Scene';
import { ObjectRegistry } from '../objects/ObjectRegistry';
import { TransformSync } from '../transforms/TransformSync';

export interface SceneManagerOptions {
  canvas: HTMLCanvasElement;
  sceneConfig?: SceneConfig;
  enableInterpolation?: boolean;
}

export type CameraViewPreset = 'perspective' | 'top' | 'front' | 'side' | 'isometric';

export class SceneManager {
  public readonly canvas: HTMLCanvasElement;
  public readonly scene: EngineeringScene;
  public readonly registry: ObjectRegistry;
  public readonly transformSync: TransformSync;

  private _renderer: THREE.WebGLRenderer;
  private _camera: THREE.PerspectiveCamera;
  private _orthoCamera: THREE.OrthographicCamera;
  private _activeCamera: THREE.Camera;
  private _orthoFrustumHeight = 10;
  private _isRunning: boolean = false;
  private _lastTimeMs: number = 0;
  private _animationFrameId: number | null = null;
  private _resizeObserver!: ResizeObserver;

  constructor(options: SceneManagerOptions) {
    this.canvas = options.canvas;
    this.scene = new EngineeringScene(options.sceneConfig);
    this.transformSync = new TransformSync(options.enableInterpolation ?? false);
    this.registry = new ObjectRegistry(this.scene.entitiesGroup, (id) => this.transformSync.cancel(id));

    const width = this.canvas.clientWidth || 800;
    const height = this.canvas.clientHeight || 600;
    const aspect = width / height;

    // Perspective Camera
    this._camera = new THREE.PerspectiveCamera(50, aspect, 0.1, 1000);
    this._camera.position.set(6, 4, 8);
    this._camera.lookAt(0, 0, 0);

    // Orthographic Camera for technical engineering blueprint views
    const frustumSize = this._orthoFrustumHeight;
    this._orthoCamera = new THREE.OrthographicCamera(
      (frustumSize * aspect) / -2,
      (frustumSize * aspect) / 2,
      frustumSize / 2,
      frustumSize / -2,
      0.1,
      1000
    );
    this._orthoCamera.position.set(0, 15, 0);
    this._orthoCamera.lookAt(0, 0, 0);

    this._activeCamera = this._camera;

    // WebGL Renderer
    this._renderer = new THREE.WebGLRenderer({
      canvas: this.canvas,
      antialias: true,
      powerPreference: 'high-performance',
      alpha: false
    });
    this._renderer.setSize(width, height, false);
    this._renderer.setPixelRatio(Math.min(window.devicePixelRatio, 2));
    this._renderer.shadowMap.enabled = true;
    this._renderer.shadowMap.type = THREE.PCFSoftShadowMap;

    this._setupResizeObserver();
  }

  public get activeCamera(): THREE.Camera {
    return this._activeCamera;
  }

  public get perspectiveCamera(): THREE.PerspectiveCamera {
    return this._camera;
  }

  public setViewPreset(preset: CameraViewPreset, target = new THREE.Vector3()): void {
    switch (preset) {
      case 'perspective':
        this._activeCamera = this._camera;
        this._camera.position.copy(target).add(new THREE.Vector3(6, 4, 8));
        this._camera.lookAt(target);
        break;
      case 'top':
        this._activeCamera = this._orthoCamera;
        this._orthoCamera.up.set(0, 0, -1);
        this._orthoCamera.position.copy(target).add(new THREE.Vector3(0, 20, 0));
        this._orthoCamera.lookAt(target);
        break;
      case 'front':
        this._activeCamera = this._camera;
        this._camera.up.set(0, 1, 0);
        this._camera.position.copy(target).add(new THREE.Vector3(0, 2, 10));
        this._camera.lookAt(target);
        break;
      case 'side':
        this._activeCamera = this._camera;
        this._camera.up.set(0, 1, 0);
        this._camera.position.copy(target).add(new THREE.Vector3(10, 2, 0));
        this._camera.lookAt(target);
        break;
      case 'isometric':
        this._activeCamera = this._camera;
        this._camera.up.set(0, 1, 0);
        this._camera.position.copy(target).add(new THREE.Vector3(8, 8, 8));
        this._camera.lookAt(target);
        break;
    }
  }

  /** Fits the active camera around registered engineering geometry and returns its target. */
  public fitToContent(padding = 1.25): THREE.Vector3 {
    if (!Number.isFinite(padding) || padding < 1 || padding > 4) {
      throw new RangeError('Viewport fit padding must be finite and between 1 and 4.');
    }
    const bounds = new THREE.Box3().setFromObject(this.scene.entitiesGroup);
    if (bounds.isEmpty()) return new THREE.Vector3(0, 0, 0);

    const center = bounds.getCenter(new THREE.Vector3());
    const size = bounds.getSize(new THREE.Vector3());
    if (this._activeCamera instanceof THREE.PerspectiveCamera) {
      const sphere = bounds.getBoundingSphere(new THREE.Sphere());
      const verticalHalfFov = THREE.MathUtils.degToRad(this._activeCamera.fov / 2);
      const horizontalHalfFov = Math.atan(Math.tan(verticalHalfFov) * this._activeCamera.aspect);
      const distance = Math.max(
        sphere.radius / Math.sin(verticalHalfFov),
        sphere.radius / Math.sin(horizontalHalfFov),
      ) * padding;
      const direction = this._activeCamera.getWorldDirection(new THREE.Vector3()).negate().normalize();
      if (direction.lengthSq() < 1e-8) direction.set(1, 0.8, 1).normalize();
      this._activeCamera.position.copy(center).addScaledVector(direction, distance);
      this._activeCamera.lookAt(center);
    } else if (this._activeCamera instanceof THREE.OrthographicCamera) {
      const aspect = Math.max(this._activeCamera.right - this._activeCamera.left, 1e-6) /
        Math.max(this._activeCamera.top - this._activeCamera.bottom, 1e-6);
      const height = Math.max(size.y, size.x / aspect, size.z / aspect, 1) * padding;
      this._setOrthographicFrustum(height);
      const direction = this._activeCamera.getWorldDirection(new THREE.Vector3()).negate().normalize();
      if (direction.lengthSq() < 1e-8) direction.set(0, 1, 0);
      this._activeCamera.position.copy(center).addScaledVector(direction, Math.max(size.length() * 2, 10));
      this._activeCamera.up.set(0, 0, -1);
      this._activeCamera.lookAt(center);
      this._activeCamera.updateProjectionMatrix();
    }
    this.renderSingleFrame();
    return center;
  }

  public start(): void {
    if (this._isRunning) return;
    this._isRunning = true;
    this._lastTimeMs = performance.now();
    this._tick(this._lastTimeMs);
  }

  public stop(): void {
    this._isRunning = false;
    if (this._animationFrameId !== null) {
      cancelAnimationFrame(this._animationFrameId);
      this._animationFrameId = null;
    }
  }

  public renderSingleFrame(): void {
    this._renderer.render(this.scene.threeScene, this._activeCamera);
  }

  private _tick = (nowMs: number): void => {
    if (!this._isRunning) return;

    const deltaSec = (nowMs - this._lastTimeMs) / 1000.0;
    this._lastTimeMs = nowMs;

    this.transformSync.update(deltaSec);
    this.renderSingleFrame();

    this._animationFrameId = requestAnimationFrame(this._tick);
  };

  private _setupResizeObserver(): void {
    this._resizeObserver = new ResizeObserver((entries) => {
      for (const entry of entries) {
        const width = entry.contentRect.width;
        const height = entry.contentRect.height;
        if (width > 0 && height > 0) {
          this.resize(width, height);
        }
      }
    });
    this._resizeObserver.observe(this.canvas);
  }

  public resize(width: number, height: number): void {
    if (!Number.isFinite(width) || !Number.isFinite(height) || width <= 0 || height <= 0) {
      throw new RangeError('Viewport dimensions must be finite and greater than zero.');
    }
    const aspect = width / height;

    this._camera.aspect = aspect;
    this._camera.updateProjectionMatrix();

    this._setOrthographicFrustum(this._orthoFrustumHeight, aspect);

    this._renderer.setSize(width, height, false);
    this.renderSingleFrame();
  }

  /** Zooms the technical orthographic view while preserving its fit across viewport resizes. */
  public zoomOrthographic(factor: number): void {
    if (!Number.isFinite(factor) || factor <= 0) throw new RangeError('Orthographic zoom factor must be finite and positive.');
    this._setOrthographicFrustum(this._orthoFrustumHeight * factor);
    this.renderSingleFrame();
  }

  public dispose(): void {
    this.stop();
    this._resizeObserver.disconnect();
    this.transformSync.clear();
    this.registry.dispose();
    this._renderer.dispose();
  }

  private _setOrthographicFrustum(height: number, aspect?: number): void {
    this._orthoFrustumHeight = THREE.MathUtils.clamp(height, 0.05, 5000);
    const viewportAspect = aspect ?? Math.max(this.canvas.clientWidth, 1) / Math.max(this.canvas.clientHeight, 1);
    this._orthoCamera.left = -(this._orthoFrustumHeight * viewportAspect) / 2;
    this._orthoCamera.right = (this._orthoFrustumHeight * viewportAspect) / 2;
    this._orthoCamera.top = this._orthoFrustumHeight / 2;
    this._orthoCamera.bottom = -this._orthoFrustumHeight / 2;
    this._orthoCamera.updateProjectionMatrix();
  }
}
