// @townbuilder/runtime -- Rust/WASM engine command protocol
// Canonical gameplay/world mutations must be expressed as Rust command enums.

export type ZoneTypeName =
  | "None"
  | "Residential"
  | "Commercial"
  | "Industrial"
  | "Civic";

export type TerrainTypeName = "Grass" | "Water" | "Sand" | "Forest" | "Rock";
export type RoadTypeName = "Local" | "Collector" | "Arterial" | "Highway";
export type SimSpeedName = "Paused" | "Slow" | "Normal" | "Fast";

export type EngineCommand =
  | {
      PlaceEntity: {
        archetype_id: number;
        x: number;
        y: number;
        rotation: number;
      };
    }
  | {
      RemoveEntity: {
        handle: {
          index: number;
          generation: number;
        };
      };
    }
  | {
      UpgradeEntity: {
        handle: {
          index: number;
          generation: number;
        };
        target_level: number;
      };
    }
  | {
      SetPolicy: {
        key:
          | "ResidentialTax"
          | "CommercialTax"
          | "IndustrialTax"
          | "PoliceBudget"
          | "FireBudget"
          | "HealthBudget"
          | "EducationBudget"
          | "TransportBudget";
        value: number;
      };
    }
  | {
      Bulldoze: {
        x: number;
        y: number;
        w: number;
        h: number;
      };
    }
  | {
      ToggleEntity: {
        handle: {
          index: number;
          generation: number;
        };
        enabled: boolean;
      };
    }
  | {
      SetZoning: {
        x: number;
        y: number;
        w: number;
        h: number;
        zone: ZoneTypeName;
      };
    }
  | {
      SetTerrain: {
        x: number;
        y: number;
        w: number;
        h: number;
        terrain: TerrainTypeName;
      };
    }
  | {
      SetRoadLine: {
        x0: number;
        y0: number;
        x1: number;
        y1: number;
        road_type: RoadTypeName;
      };
    }
  | {
      RemoveRoad: {
        x: number;
        y: number;
      };
    }
  | {
      SetSimSpeed: {
        speed: SimSpeedName;
      };
    };
