/**
 * HELQIX Visual Runtime — Scene
 * Layer 14: 14-visual-runtime
 *
 * Configures the physical engineering 3D scene environment.
 * Sets up ground grid, lighting calibrated for CAD/mechanical inspection, and metric reference axes.
 */

import * as THREE from 'three';

export interface SceneConfig {
  backgroundColor?: number;
  showGrid?: boolean;
  showAxes?: boolean;
  gridSizeMeters?: number;
}

export class EngineeringScene {
  public readonly threeScene: THREE.Scene;
  public readonly entitiesGroup: THREE.Group;
  public readonly environmentGroup: THREE.Group;

  private _gridHelper?: THREE.GridHelper;
  private _axesHelper?: THREE.AxesHelper;

  constructor(config: SceneConfig = {}) {
    this.threeScene = new THREE.Scene();
    const backgroundColor = config.backgroundColor ?? 0x11120f;
    this.threeScene.background = new THREE.Color(backgroundColor);

    // Fog for depth in large CAD viewports
    this.threeScene.fog = new THREE.FogExp2(backgroundColor, 0.015);

    this.environmentGroup = new THREE.Group();
    this.environmentGroup.name = 'EnvironmentGroup';
    this.threeScene.add(this.environmentGroup);

    this.entitiesGroup = new THREE.Group();
    this.entitiesGroup.name = 'EngineeringEntities';
    this.threeScene.add(this.entitiesGroup);

    this._setupLighting();
    this._setupEnvironment(config);
  }

  private _setupLighting(): void {
    // Ambient light for baseline visibility
    const ambientLight = new THREE.AmbientLight(0xffffff, 0.7);
    this.threeScene.add(ambientLight);

    // Key directional light with shadow mapping
    const keyLight = new THREE.DirectionalLight(0xffffff, 1.2);
    keyLight.position.set(10, 20, 15);
    keyLight.castShadow = true;
    keyLight.shadow.mapSize.width = 2048;
    keyLight.shadow.mapSize.height = 2048;
    keyLight.shadow.bias = -0.0001;
    this.threeScene.add(keyLight);

    // Fill light from opposite side
    const fillLight = new THREE.DirectionalLight(0xc2ed74, 0.34);
    fillLight.position.set(-15, 10, -10);
    this.threeScene.add(fillLight);

    // Soft ground rim light
    const rimLight = new THREE.HemisphereLight(0xffffff, 0x1e293b, 0.5);
    this.threeScene.add(rimLight);
  }

  private _setupEnvironment(config: SceneConfig): void {
    const size = config.gridSizeMeters ?? 50;

    if (config.showGrid !== false) {
      this._gridHelper = new THREE.GridHelper(size, 50, 0x69774c, 0x2d3127);
      this._gridHelper.position.y = 0;
      this.environmentGroup.add(this._gridHelper);
    }

    if (config.showAxes !== false) {
      this._axesHelper = new THREE.AxesHelper(2.5);
      this._axesHelper.position.set(0, 0.01, 0);
      this.environmentGroup.add(this._axesHelper);
    }
  }

  public setGridVisibility(visible: boolean): void {
    if (this._gridHelper) this._gridHelper.visible = visible;
  }

  public setAxesVisibility(visible: boolean): void {
    if (this._axesHelper) this._axesHelper.visible = visible;
  }
}
