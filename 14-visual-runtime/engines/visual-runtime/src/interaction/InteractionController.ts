/**
 * HELQIX Visual Runtime — InteractionController
 * Layer 14: 14-visual-runtime
 *
 * Provides smooth CAD orbit/pan/zoom viewport navigation and raycast component selection.
 * Dispatches interaction events to parent UI listeners.
 */

import * as THREE from 'three';
import { SceneManager } from '../scene/SceneManager';
import { EngineeringObject } from '../objects/EngineeringObject';

export type SelectionCallback = (object: EngineeringObject | null) => void;

export class InteractionController {
  private _manager: SceneManager;
  private _canvas: HTMLCanvasElement;
  private _raycaster: THREE.Raycaster;
  private _mousePos: THREE.Vector2;

  private _isDragging: boolean = false;
  private _isPanning: boolean = false;
  private _previousMouseX: number = 0;
  private _previousMouseY: number = 0;
  private _draggedDuringPointer: boolean = false;

  private _cameraTarget: THREE.Vector3 = new THREE.Vector3(0, 0, 0);
  private _spherical: THREE.Spherical = new THREE.Spherical(12, Math.PI / 3, Math.PI / 4);

  public onSelectObject?: SelectionCallback;
  public onHoverObject?: SelectionCallback;

  private _onContextMenu = (event: MouseEvent): void => event.preventDefault();

  private _selectedObjectId: string | null = null;
  private _hoveredObjectId: string | null = null;

  constructor(manager: SceneManager) {
    this._manager = manager;
    this._canvas = manager.canvas;
    this._raycaster = new THREE.Raycaster();
    this._mousePos = new THREE.Vector2();

    this._bindEvents();
    this.syncCameraTarget();
  }

  private _bindEvents(): void {
    this._canvas.addEventListener('mousedown', this._onMouseDown);
    window.addEventListener('mousemove', this._onMouseMove);
    window.addEventListener('mouseup', this._onMouseUp);
    this._canvas.addEventListener('wheel', this._onWheel, { passive: false });
    this._canvas.addEventListener('click', this._onClick);
    this._canvas.addEventListener('contextmenu', this._onContextMenu);
  }

  /** Keeps orbit and pan controls aligned after a camera preset or fit operation. */
  public syncCameraTarget(target: THREE.Vector3 = new THREE.Vector3()): void {
    this._cameraTarget.copy(target);
    const offset = this._manager.activeCamera.position.clone().sub(target);
    if (offset.lengthSq() > 1e-8) this._spherical.setFromVector3(offset);
  }

  public dispose(): void {
    this._canvas.removeEventListener('mousedown', this._onMouseDown);
    window.removeEventListener('mousemove', this._onMouseMove);
    window.removeEventListener('mouseup', this._onMouseUp);
    this._canvas.removeEventListener('wheel', this._onWheel);
    this._canvas.removeEventListener('click', this._onClick);
    this._canvas.removeEventListener('contextmenu', this._onContextMenu);
  }

  private _onMouseDown = (e: MouseEvent): void => {
    this._draggedDuringPointer = false;
    if (e.button === 0) {
      this._isDragging = true;
    } else if (e.button === 2) {
      this._isPanning = true;
    }
    this._previousMouseX = e.clientX;
    this._previousMouseY = e.clientY;
  };

  private _onMouseMove = (e: MouseEvent): void => {
    const deltaX = e.clientX - this._previousMouseX;
    const deltaY = e.clientY - this._previousMouseY;
    if ((this._isDragging || this._isPanning) && (Math.abs(deltaX) > 3 || Math.abs(deltaY) > 3)) {
      this._draggedDuringPointer = true;
    }
    this._previousMouseX = e.clientX;
    this._previousMouseY = e.clientY;

    if (this._isDragging) {
      // Orbit rotation
      this._spherical.theta -= deltaX * 0.008;
      this._spherical.phi = Math.max(0.05, Math.min(Math.PI / 2 - 0.02, this._spherical.phi - deltaY * 0.008));
      this._updateCameraFromSpherical();
    } else if (this._isPanning) {
      // Pan translation in camera plane
      const panSpeed = 0.006 * this._spherical.radius;
      const camera = this._manager.activeCamera;
      const right = new THREE.Vector3(1, 0, 0).applyQuaternion(camera.quaternion);
      const up = new THREE.Vector3(0, 1, 0).applyQuaternion(camera.quaternion);

      this._cameraTarget.addScaledVector(right, -deltaX * panSpeed);
      this._cameraTarget.addScaledVector(up, deltaY * panSpeed);
      this._updateCameraFromSpherical();
    } else {
      // Hover raycasting
      this._updateNormalizedMouse(e);
      const hit = this._raycastUnderMouse();
      if (hit?.id !== this._hoveredObjectId) {
        this._hoveredObjectId = hit?.id || null;
        if (this.onHoverObject) {
          this.onHoverObject(hit);
        }
      }
    }
  };

  private _onMouseUp = (): void => {
    this._isDragging = false;
    this._isPanning = false;
  };

  private _onWheel = (e: WheelEvent): void => {
    e.preventDefault();
    const zoomFactor = e.deltaY > 0 ? 1.1 : 0.9;
    if (this._manager.activeCamera instanceof THREE.OrthographicCamera) {
      this._manager.zoomOrthographic(zoomFactor);
    } else {
      this._spherical.radius = Math.max(1.5, Math.min(80, this._spherical.radius * zoomFactor));
      this._updateCameraFromSpherical();
    }
  };

  private _onClick = (e: MouseEvent): void => {
    if (this._draggedDuringPointer) {
      this._draggedDuringPointer = false;
      return;
    }
    this._updateNormalizedMouse(e);
    const hit = this._raycastUnderMouse();

    if (this._selectedObjectId) {
      const prev = this._manager.registry.get(this._selectedObjectId);
      if (prev) prev.applyAttributes({ selected: false });
    }

    if (hit) {
      hit.applyAttributes({ selected: true });
      this._selectedObjectId = hit.id;
      if (this.onSelectObject) this.onSelectObject(hit);
    } else {
      this._selectedObjectId = null;
      if (this.onSelectObject) this.onSelectObject(null);
    }

    this._manager.renderSingleFrame();
  };

  private _updateNormalizedMouse(e: MouseEvent): void {
    const rect = this._canvas.getBoundingClientRect();
    this._mousePos.x = ((e.clientX - rect.left) / rect.width) * 2 - 1;
    this._mousePos.y = -((e.clientY - rect.top) / rect.height) * 2 + 1;
  }

  private _raycastUnderMouse(): EngineeringObject | null {
    this._raycaster.setFromCamera(this._mousePos, this._manager.activeCamera);
    const intersects = this._raycaster.intersectObjects(this._manager.scene.entitiesGroup.children, true);

    if (intersects.length > 0) {
      const hitObj = this._manager.registry.findByRaycastHit(intersects[0].object);
      return hitObj || null;
    }
    return null;
  }

  private _updateCameraFromSpherical(): void {
    const camera = this._manager.activeCamera;
    const offset = new THREE.Vector3().setFromSpherical(this._spherical);
    camera.position.copy(this._cameraTarget).add(offset);
    camera.lookAt(this._cameraTarget);
    this._manager.renderSingleFrame();
  }

  public focusOnObject(id: string): void {
    const obj = this._manager.registry.get(id);
    if (!obj) return;

    this._cameraTarget.copy(obj.root3D.position);
    this._spherical.radius = 4.0;
    this._updateCameraFromSpherical();
  }
}
