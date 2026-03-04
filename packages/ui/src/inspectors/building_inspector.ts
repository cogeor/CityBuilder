/** Building info data */
export interface BuildingInfo {
  entityId: number;
  name: string;
  archetypeId: number;
  level: number;
  maxLevel: number;
  /** Current residents or workers */
  occupants: number;
  /** Maximum capacity */
  capacity: number;
  /** Is power connected */
  hasPower: boolean;
  /** Is water connected */
  hasWater: boolean;
  /** Is fully staffed */
  isStaffed: boolean;
  /** Is under construction */
  underConstruction: boolean;
  /** Construction progress (0-100%) */
  constructionPercent: number;
  /** Monthly upkeep in cents */
  upkeepCents: number;
  /** Is on fire */
  onFire: boolean;
  /** Power demand in kW */
  powerDemandKw: number;
  /** Power supply in kW (for power plants) */
  powerSupplyKw: number;
  /** Water demand */
  waterDemand: number;
  /** Water supply (for water buildings) */
  waterSupply: number;
}

/** Tile info data */
export interface TileInfo {
  x: number;
  y: number;
  terrainType: string;
  elevation: number;
  zoneType: string;
  hasRoad: boolean;
  roadType: string;
  pollution: number;    // 0-100
  crime: number;        // 0-100
  landValue: number;    // 0-100
  noise: number;        // 0-100
  desirability: number; // 0-100
}

/** District summary info */
export interface DistrictInfo {
  name: string;
  population: number;
  employment: number;
  unemploymentRate: number;  // 0-100
  averageLandValue: number;
  totalTax: number;
  totalUpkeep: number;
}

/** Inspector panel types */
export enum InspectorType {
  None = 'none',
  Building = 'building',
  Tile = 'tile',
  District = 'district',
}

/** Inspector event types */
export type InspectorEventType = 'open' | 'close' | 'pin' | 'unpin' | 'upgrade' | 'demolish';
export type InspectorEventHandler = (type: InspectorEventType, data: any) => void;

/**
 * InspectorManager — manages building/tile/district inspectors.
 *
 * Click on entity -> open building inspector
 * Click on empty tile -> open tile inspector
 * Close button or click elsewhere -> close (unless pinned)
 */
export class InspectorManager {
  private activeType: InspectorType;
  private buildingInfo: BuildingInfo | null;
  private tileInfo: TileInfo | null;
  private districtInfo: DistrictInfo | null;
  private pinned: boolean;
  private eventHandlers: InspectorEventHandler[];

  constructor() {
    this.activeType = InspectorType.None;
    this.buildingInfo = null;
    this.tileInfo = null;
    this.districtInfo = null;
    this.pinned = false;
    this.eventHandlers = [];
  }

  // --- Getters ---
  getActiveType(): InspectorType { return this.activeType; }
  getBuildingInfo(): BuildingInfo | null { return this.buildingInfo ? { ...this.buildingInfo } : null; }
  getTileInfo(): TileInfo | null { return this.tileInfo ? { ...this.tileInfo } : null; }
  getDistrictInfo(): DistrictInfo | null { return this.districtInfo ? { ...this.districtInfo } : null; }
  isPinned(): boolean { return this.pinned; }

  // --- Actions ---
  inspectBuilding(info: BuildingInfo): void {
    this.activeType = InspectorType.Building;
    this.buildingInfo = { ...info };
    this.tileInfo = null;
    this.districtInfo = null;
    this.emit('open', { type: InspectorType.Building, info });
  }

  inspectTile(info: TileInfo): void {
    this.activeType = InspectorType.Tile;
    this.tileInfo = { ...info };
    this.buildingInfo = null;
    this.districtInfo = null;
    this.emit('open', { type: InspectorType.Tile, info });
  }

  inspectDistrict(info: DistrictInfo): void {
    this.activeType = InspectorType.District;
    this.districtInfo = { ...info };
    this.buildingInfo = null;
    this.tileInfo = null;
    this.emit('open', { type: InspectorType.District, info });
  }

  updateBuildingInfo(info: Partial<BuildingInfo>): void {
    if (this.buildingInfo) {
      Object.assign(this.buildingInfo, info);
    }
  }

  updateTileInfo(info: Partial<TileInfo>): void {
    if (this.tileInfo) {
      Object.assign(this.tileInfo, info);
    }
  }

  close(): void {
    if (this.pinned) return;
    this.activeType = InspectorType.None;
    this.buildingInfo = null;
    this.tileInfo = null;
    this.districtInfo = null;
    this.emit('close', {});
  }

  forceClose(): void {
    this.pinned = false;
    this.activeType = InspectorType.None;
    this.buildingInfo = null;
    this.tileInfo = null;
    this.districtInfo = null;
    this.emit('close', {});
  }

  pin(): void {
    this.pinned = true;
    this.emit('pin', {});
  }

  unpin(): void {
    this.pinned = false;
    this.emit('unpin', {});
  }

  togglePin(): void {
    if (this.pinned) this.unpin();
    else this.pin();
  }

  /** Request building upgrade (emits event for command generation) */
  requestUpgrade(): void {
    if (this.buildingInfo) {
      this.emit('upgrade', { entityId: this.buildingInfo.entityId });
    }
  }

  /** Request building demolition (emits event for command generation) */
  requestDemolish(): void {
    if (this.buildingInfo) {
      this.emit('demolish', { entityId: this.buildingInfo.entityId });
    }
  }

  // --- Display Helpers ---
  /** Format upkeep as monthly cost string */
  formatUpkeep(centsPerTick: number): string {
    // ~72000 ticks/month (20 ticks/sec * 3600 sec)
    const monthly = (centsPerTick * 72000) / 100;
    if (monthly >= 1000) return `$${(monthly / 1000).toFixed(1)}K/mo`;
    return `$${monthly.toFixed(0)}/mo`;
  }

  /** Format capacity as "X / Y" */
  formatCapacity(current: number, max: number): string {
    return `${current} / ${max}`;
  }

  /** Format level */
  formatLevel(level: number, maxLevel: number): string {
    return `Level ${level} / ${maxLevel}`;
  }

  /** Format percentage */
  formatPercent(value: number): string {
    return `${Math.round(value)}%`;
  }

  /** Get status summary for a building */
  getBuildingStatus(info: BuildingInfo): string[] {
    const statuses: string[] = [];
    if (info.underConstruction) statuses.push(`Building... ${Math.round(info.constructionPercent)}%`);
    if (info.onFire) statuses.push('On Fire!');
    if (!info.hasPower) statuses.push('No Power');
    if (!info.hasWater) statuses.push('No Water');
    if (!info.isStaffed && !info.underConstruction) statuses.push('Understaffed');
    if (statuses.length === 0) statuses.push('Operating');
    return statuses;
  }

  // --- Events ---
  addEventListener(handler: InspectorEventHandler): void {
    this.eventHandlers.push(handler);
  }

  removeEventListener(handler: InspectorEventHandler): void {
    const idx = this.eventHandlers.indexOf(handler);
    if (idx >= 0) this.eventHandlers.splice(idx, 1);
  }

  private emit(type: InspectorEventType, data: any): void {
    for (const handler of this.eventHandlers) {
      handler(type, data);
    }
  }
}
