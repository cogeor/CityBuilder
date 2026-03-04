// @townbuilder/ui — User interface components

export {
  SimSpeed,
  ToolType,
  type CityStats,
  type Notification,
  type PanelState,
  type ShellEventType,
  type ShellEventHandler,
  DEFAULT_CITY_STATS,
  HudShell,
} from './shell/index.js';

export {
  InspectorType,
  type BuildingInfo,
  type TileInfo,
  type DistrictInfo,
  type InspectorEventType,
  type InspectorEventHandler,
  InspectorManager,
  TaxCategory,
  ExpenseDepartment,
  BudgetViewMode,
  type IncomeItem,
  type ExpenseItem,
  type BudgetSnapshot,
  type BudgetEventType,
  type BudgetEventHandler,
  BudgetPanel,
} from './inspectors/index.js';

export {
  ToolState,
  PlacementValidity,
  type TileCoord,
  type ToolCommand,
  type DragRect,
  type ValidateCallback,
  type CostCallback,
  type ToolEventType,
  type ToolEventHandler,
  ToolManager,
} from './tools/index.js';

export {
  UIOverlayType,
  type OverlayButton,
  type LegendEntry,
  type OverlayLegend,
  type MinimapConfig,
  type MinimapViewport,
  type OverlayPanelEventType,
  type OverlayPanelEventHandler,
  OVERLAY_BUTTONS,
  OVERLAY_LEGENDS,
  OverlayPanel,
} from './overlays/index.js';
