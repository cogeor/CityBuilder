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
  AdvisorCategory,
  AdvisorSeverity,
  type DiagnosticItem,
  type AdvisorState,
  type CityMetrics,
  formatSeverityLabel,
  AdvisorPanel,
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

export {
  type CameraState,
  type KeyBindings,
  type CameraLimits,
  type CameraEventType,
  type CameraEventHandler,
  DEFAULT_KEY_BINDINGS,
  DEFAULT_CAMERA_LIMITS,
  CameraController,
} from './input/index.js';

export {
  DevPanel,
  type PerformanceMetrics,
  type EntityDebugInfo,
  type CacheStats,
  type PhaseWheelStatus,
  type DevToolsEventType,
  type DevToolsEventHandler,
  type ConsoleEntry,
  DevTools,
} from './devtools/index.js';
