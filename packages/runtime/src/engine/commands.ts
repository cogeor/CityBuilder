// @townbuilder/runtime -- Rust/WASM engine command protocol
// Canonical gameplay/world mutations must be expressed as Rust command enums.

export type ZoneTypeName =
  | "None"
  | "Residential"
  | "Commercial"
  | "Industrial"
  | "Civic";

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
    };
