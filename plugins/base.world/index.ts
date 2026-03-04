// @townbuilder/base.world — Barrel export
export {
  type MapSizePreset,
  type CountryPreset,
  type TimePreset,
  type WorldDefaults,
  MAP_SIZES,
  COUNTRY_PRESETS,
  TIME_DEFAULTS,
  WORLD_DEFAULTS,
  getMapSize,
  getCountryPreset,
} from "./presets.js";

export {
  EraPreset,
  type StartProfile,
  START_PROFILES,
  getProfileById,
  getProfilesForMapSize,
  getProfilesForEra,
  validateProfileCompatibility,
} from "./start_profiles.js";
