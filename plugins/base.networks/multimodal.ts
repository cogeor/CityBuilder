/** Transit mode types */
export enum TransitMode {
  Bus = "bus",
  Tram = "tram",
  Metro = "metro",
  CommuterRail = "commuter_rail",
}

/** Transit vehicle specification */
export interface TransitVehicle {
  mode: TransitMode;
  name: string;
  capacity: number;
  speedKmH: number;
  costPerKm: number;
  frequency: number;  // minutes between vehicles
}

export const TRANSIT_VEHICLES: TransitVehicle[] = [
  { mode: TransitMode.Bus, name: "City Bus", capacity: 50, speedKmH: 25, costPerKm: 3, frequency: 10 },
  { mode: TransitMode.Tram, name: "Light Rail", capacity: 150, speedKmH: 35, costPerKm: 8, frequency: 8 },
  { mode: TransitMode.Metro, name: "Metro Train", capacity: 800, speedKmH: 60, costPerKm: 15, frequency: 5 },
  { mode: TransitMode.CommuterRail, name: "Commuter Rail", capacity: 1200, speedKmH: 80, costPerKm: 20, frequency: 15 },
];

/** Get all vehicles for a given transit mode */
export function getVehiclesByMode(mode: TransitMode): TransitVehicle[] {
  return TRANSIT_VEHICLES.filter(v => v.mode === mode);
}

/** Estimate passengers per hour for a line */
export function estimateLineCapacity(vehicle: TransitVehicle, routeLengthKm: number): number {
  // Round-trip time in minutes
  const roundTripMinutes = (routeLengthKm * 2) / (vehicle.speedKmH / 60);
  // Vehicles needed to maintain frequency
  const vehiclesNeeded = Math.ceil(roundTripMinutes / vehicle.frequency);
  // Trips per hour per direction
  const tripsPerHour = 60 / vehicle.frequency;
  return Math.floor(tripsPerHour * vehicle.capacity);
}

/** Estimate annual operating cost for a line in cost units */
export function estimateLineCost(vehicle: TransitVehicle, routeLengthKm: number): number {
  // Daily trips (both directions) * 18 operating hours
  const tripsPerHour = 60 / vehicle.frequency;
  const dailyTrips = tripsPerHour * 18 * 2; // both directions, 18h day
  const dailyKm = dailyTrips * routeLengthKm;
  const dailyCost = dailyKm * vehicle.costPerKm;
  // Annual: 365 days
  return Math.floor(dailyCost * 365);
}
