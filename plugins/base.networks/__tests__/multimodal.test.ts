import { describe, it, expect } from "vitest";
import {
  TransitMode,
  TRANSIT_VEHICLES,
  getVehiclesByMode,
  estimateLineCapacity,
  estimateLineCost,
} from "../multimodal.js";

// ─── TRANSIT_VEHICLES ───────────────────────────────────────────────────────

describe("TRANSIT_VEHICLES", () => {
  it("has 4 entries", () => {
    expect(TRANSIT_VEHICLES).toHaveLength(4);
  });

  it("all vehicles have positive capacity", () => {
    for (const v of TRANSIT_VEHICLES) {
      expect(v.capacity).toBeGreaterThan(0);
    }
  });

  it("all vehicles have positive speed", () => {
    for (const v of TRANSIT_VEHICLES) {
      expect(v.speedKmH).toBeGreaterThan(0);
    }
  });

  it("all vehicles have positive costPerKm", () => {
    for (const v of TRANSIT_VEHICLES) {
      expect(v.costPerKm).toBeGreaterThan(0);
    }
  });

  it("all vehicles have positive frequency", () => {
    for (const v of TRANSIT_VEHICLES) {
      expect(v.frequency).toBeGreaterThan(0);
    }
  });
});

// ─── getVehiclesByMode ──────────────────────────────────────────────────────

describe("getVehiclesByMode", () => {
  it("filters Bus mode correctly", () => {
    const buses = getVehiclesByMode(TransitMode.Bus);
    expect(buses).toHaveLength(1);
    expect(buses[0].name).toBe("City Bus");
  });

  it("filters Metro mode correctly", () => {
    const metros = getVehiclesByMode(TransitMode.Metro);
    expect(metros).toHaveLength(1);
    expect(metros[0].capacity).toBe(800);
  });

  it("filters Tram mode correctly", () => {
    const trams = getVehiclesByMode(TransitMode.Tram);
    expect(trams).toHaveLength(1);
    expect(trams[0].name).toBe("Light Rail");
  });

  it("filters CommuterRail mode correctly", () => {
    const rails = getVehiclesByMode(TransitMode.CommuterRail);
    expect(rails).toHaveLength(1);
    expect(rails[0].speedKmH).toBe(80);
  });
});

// ─── estimateLineCapacity ───────────────────────────────────────────────────

describe("estimateLineCapacity", () => {
  it("returns a positive number for valid input", () => {
    const bus = TRANSIT_VEHICLES[0]; // City Bus
    const cap = estimateLineCapacity(bus, 10);
    expect(cap).toBeGreaterThan(0);
  });

  it("capacity increases with vehicle capacity", () => {
    const bus = TRANSIT_VEHICLES[0];  // 50 capacity, 10 min
    const metro = TRANSIT_VEHICLES[2]; // 800 capacity, 5 min
    const busCap = estimateLineCapacity(bus, 10);
    const metroCap = estimateLineCapacity(metro, 10);
    expect(metroCap).toBeGreaterThan(busCap);
  });

  it("bus line capacity for 10km route", () => {
    const bus = TRANSIT_VEHICLES[0]; // frequency=10, capacity=50
    const cap = estimateLineCapacity(bus, 10);
    // 60/10 = 6 trips per hour, 6 * 50 = 300
    expect(cap).toBe(300);
  });
});

// ─── estimateLineCost ───────────────────────────────────────────────────────

describe("estimateLineCost", () => {
  it("returns a positive number for valid input", () => {
    const bus = TRANSIT_VEHICLES[0];
    const cost = estimateLineCost(bus, 10);
    expect(cost).toBeGreaterThan(0);
  });

  it("cost increases with route length", () => {
    const bus = TRANSIT_VEHICLES[0];
    const costShort = estimateLineCost(bus, 5);
    const costLong = estimateLineCost(bus, 20);
    expect(costLong).toBeGreaterThan(costShort);
  });

  it("cost increases with costPerKm", () => {
    const bus = TRANSIT_VEHICLES[0];  // costPerKm=3
    const metro = TRANSIT_VEHICLES[2]; // costPerKm=15
    const busCost = estimateLineCost(bus, 10);
    const metroCost = estimateLineCost(metro, 10);
    expect(metroCost).toBeGreaterThan(busCost);
  });
});
