//! Multi-modal transit network system.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransitType {
    Bus = 0,
    Tram = 1,
    Metro = 2,
    CommuterRail = 3,
}

#[derive(Debug, Clone)]
pub struct TransitLine {
    pub id: u16,
    pub transit_type: TransitType,
    pub stops: Vec<(u16, u16)>, // (x, y) coordinates
    pub frequency_minutes: u16,
    pub capacity_per_vehicle: u16,
}

#[derive(Debug, Clone, Default)]
pub struct TransitStats {
    pub total_lines: u32,
    pub total_ridership: u32,
    pub total_revenue: i64,
    pub total_operating_cost: i64,
    pub mode_split_transit: f32, // fraction using transit vs car
}

/// Trait for pluggable transit network evaluation.
pub trait ITransitNetwork {
    fn evaluate_routes(&self, lines: &[TransitLine]) -> Vec<RouteMetrics>;
    fn compute_ridership(&self, line: &TransitLine, demand: u32) -> u32;
    fn get_mode_split(&self, transit_time: u32, car_time: u32) -> f32;
    fn name(&self) -> &str;
}

#[derive(Debug, Clone)]
pub struct RouteMetrics {
    pub line_id: u16,
    pub route_length_km: f32,
    pub estimated_ridership: u32,
    pub revenue_per_tick: i64,
    pub cost_per_tick: i64,
}

pub struct DefaultTransitNetwork {
    pub fare_per_ride: i64,    // in cents
    pub cost_per_km: [i64; 4], // per transit type
    pub speed_kmh: [u16; 4],  // per transit type
}

impl Default for DefaultTransitNetwork {
    fn default() -> Self {
        Self {
            fare_per_ride: 250, // $2.50
            cost_per_km: [300, 800, 1500, 2000],
            speed_kmh: [25, 35, 60, 80],
        }
    }
}

impl ITransitNetwork for DefaultTransitNetwork {
    fn evaluate_routes(&self, lines: &[TransitLine]) -> Vec<RouteMetrics> {
        lines
            .iter()
            .map(|line| {
                let length = compute_route_length(&line.stops);
                let type_idx = line.transit_type as usize;
                let cost = (length * self.cost_per_km[type_idx] as f32) as i64;
                RouteMetrics {
                    line_id: line.id,
                    route_length_km: length,
                    estimated_ridership: line.capacity_per_vehicle as u32 * 60
                        / line.frequency_minutes.max(1) as u32,
                    revenue_per_tick: 0, // computed from actual ridership
                    cost_per_tick: cost,
                }
            })
            .collect()
    }

    fn compute_ridership(&self, line: &TransitLine, demand: u32) -> u32 {
        let vehicles_per_hour = 60 / line.frequency_minutes.max(1) as u32;
        let hourly_capacity = vehicles_per_hour * line.capacity_per_vehicle as u32;
        demand.min(hourly_capacity)
    }

    fn get_mode_split(&self, transit_time: u32, car_time: u32) -> f32 {
        if transit_time == 0 && car_time == 0 {
            return 0.5;
        }
        if transit_time == 0 {
            return 1.0;
        }
        if car_time == 0 {
            return 0.0;
        }
        let ratio = car_time as f32 / transit_time as f32;
        (ratio / (1.0 + ratio)).clamp(0.0, 1.0)
    }

    fn name(&self) -> &str {
        "default_transit"
    }
}

/// Compute route length in km from stop coordinates (1 tile = ~10m).
pub fn compute_route_length(stops: &[(u16, u16)]) -> f32 {
    if stops.len() < 2 {
        return 0.0;
    }
    let mut total = 0.0f32;
    for i in 1..stops.len() {
        let dx = stops[i].0 as f32 - stops[i - 1].0 as f32;
        let dy = stops[i].1 as f32 - stops[i - 1].1 as f32;
        total += (dx * dx + dy * dy).sqrt() * 0.01; // tiles to km
    }
    total
}

/// Compute fare revenue from ridership.
pub fn compute_fare_revenue(ridership: u32, fare: i64) -> i64 {
    ridership as i64 * fare
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: create a simple bus line with given stops.
    fn make_bus_line(id: u16, stops: Vec<(u16, u16)>) -> TransitLine {
        TransitLine {
            id,
            transit_type: TransitType::Bus,
            stops,
            frequency_minutes: 10,
            capacity_per_vehicle: 50,
        }
    }

    // ─── Test 1: compute_route_length with 2 stops ──────────────────────

    #[test]
    fn route_length_two_stops() {
        // Two stops at (0,0) and (100,0) => distance = 100 tiles * 0.01 = 1.0 km
        let stops = vec![(0, 0), (100, 0)];
        let length = compute_route_length(&stops);
        assert!((length - 1.0).abs() < 0.001, "Expected ~1.0 km, got {}", length);
    }

    // ─── Test 2: compute_route_length with 0 stops returns 0 ────────────

    #[test]
    fn route_length_zero_stops() {
        let stops: Vec<(u16, u16)> = vec![];
        assert_eq!(compute_route_length(&stops), 0.0);
    }

    // ─── Test 3: compute_route_length with 1 stop returns 0 ─────────────

    #[test]
    fn route_length_one_stop() {
        let stops = vec![(10, 20)];
        assert_eq!(compute_route_length(&stops), 0.0);
    }

    // ─── Test 4: compute_ridership capped by capacity ───────────────────

    #[test]
    fn ridership_capped_by_capacity() {
        let network = DefaultTransitNetwork::default();
        let line = make_bus_line(1, vec![(0, 0), (100, 0)]);
        // frequency=10 min => 6 vehicles/hour, capacity 50 each => 300/hour
        let ridership = network.compute_ridership(&line, 200);
        assert_eq!(ridership, 200); // demand 200 < capacity 300
    }

    // ─── Test 5: compute_ridership with high demand => max capacity ─────

    #[test]
    fn ridership_high_demand_max_capacity() {
        let network = DefaultTransitNetwork::default();
        let line = make_bus_line(1, vec![(0, 0), (100, 0)]);
        // 6 vehicles/hour * 50 capacity = 300 max
        let ridership = network.compute_ridership(&line, 1000);
        assert_eq!(ridership, 300); // capped at hourly capacity
    }

    // ─── Test 6: get_mode_split equal times => ~0.5 ─────────────────────

    #[test]
    fn mode_split_equal_times() {
        let network = DefaultTransitNetwork::default();
        let split = network.get_mode_split(30, 30);
        // ratio = 1.0, split = 1.0 / 2.0 = 0.5
        assert!((split - 0.5).abs() < 0.001, "Expected ~0.5, got {}", split);
    }

    // ─── Test 7: get_mode_split transit faster => higher split ──────────

    #[test]
    fn mode_split_transit_faster() {
        let network = DefaultTransitNetwork::default();
        // Transit takes 10 min, car takes 30 min => ratio = 3.0
        let split = network.get_mode_split(10, 30);
        // split = 3.0 / 4.0 = 0.75
        assert!((split - 0.75).abs() < 0.001, "Expected ~0.75, got {}", split);
    }

    // ─── Test 8: evaluate_routes processes all lines ────────────────────

    #[test]
    fn evaluate_routes_processes_all_lines() {
        let network = DefaultTransitNetwork::default();
        let lines = vec![
            make_bus_line(1, vec![(0, 0), (100, 0)]),
            TransitLine {
                id: 2,
                transit_type: TransitType::Metro,
                stops: vec![(0, 0), (200, 0)],
                frequency_minutes: 5,
                capacity_per_vehicle: 200,
            },
        ];

        let metrics = network.evaluate_routes(&lines);
        assert_eq!(metrics.len(), 2);
        assert_eq!(metrics[0].line_id, 1);
        assert_eq!(metrics[1].line_id, 2);
    }

    // ─── Test 9: compute_fare_revenue correct ───────────────────────────

    #[test]
    fn fare_revenue_correct() {
        let revenue = compute_fare_revenue(100, 250);
        assert_eq!(revenue, 25_000); // 100 rides * $2.50 = $25,000 in cents
    }

    // ─── Test 10: Default costs reasonable ──────────────────────────────

    #[test]
    fn default_costs_reasonable() {
        let network = DefaultTransitNetwork::default();
        assert_eq!(network.fare_per_ride, 250);
        // Bus cheapest, commuter rail most expensive
        assert!(network.cost_per_km[0] < network.cost_per_km[3]);
        // Bus slowest, commuter rail fastest
        assert!(network.speed_kmh[0] < network.speed_kmh[3]);
        // All costs positive
        for &c in &network.cost_per_km {
            assert!(c > 0);
        }
        // All speeds positive
        for &s in &network.speed_kmh {
            assert!(s > 0);
        }
    }

    // ─── Test 11: TransitType discriminants ─────────────────────────────

    #[test]
    fn transit_type_discriminants() {
        assert_eq!(TransitType::Bus as u8, 0);
        assert_eq!(TransitType::Tram as u8, 1);
        assert_eq!(TransitType::Metro as u8, 2);
        assert_eq!(TransitType::CommuterRail as u8, 3);
    }

    // ─── Test 12: Model name correct ────────────────────────────────────

    #[test]
    fn model_name_correct() {
        let network = DefaultTransitNetwork::default();
        assert_eq!(network.name(), "default_transit");
    }

    // ─── Test 13: get_mode_split edge cases ─────────────────────────────

    #[test]
    fn mode_split_edge_cases() {
        let network = DefaultTransitNetwork::default();
        // Both zero => 0.5
        assert_eq!(network.get_mode_split(0, 0), 0.5);
        // Transit zero, car nonzero => 1.0 (transit infinitely fast)
        assert_eq!(network.get_mode_split(0, 30), 1.0);
        // Car zero, transit nonzero => 0.0 (car infinitely fast)
        assert_eq!(network.get_mode_split(30, 0), 0.0);
    }

    // ─── Test 14: evaluate_routes cost scales with route length ─────────

    #[test]
    fn evaluate_routes_cost_scales_with_length() {
        let network = DefaultTransitNetwork::default();
        let short_line = make_bus_line(1, vec![(0, 0), (50, 0)]);
        let long_line = make_bus_line(2, vec![(0, 0), (200, 0)]);

        let metrics = network.evaluate_routes(&[short_line, long_line]);
        assert!(metrics[0].cost_per_tick < metrics[1].cost_per_tick);
    }

    // ─── Test 15: route_length_multi_stops ──────────────────────────────

    #[test]
    fn route_length_multi_stops() {
        // Three stops forming an L-shape: (0,0) -> (100,0) -> (100,100)
        // Segment 1: 100 tiles = 1.0 km
        // Segment 2: 100 tiles = 1.0 km
        // Total: 2.0 km
        let stops = vec![(0, 0), (100, 0), (100, 100)];
        let length = compute_route_length(&stops);
        assert!((length - 2.0).abs() < 0.001, "Expected ~2.0 km, got {}", length);
    }

    // ─── Test 16: TransitStats default ──────────────────────────────────

    #[test]
    fn transit_stats_default() {
        let stats = TransitStats::default();
        assert_eq!(stats.total_lines, 0);
        assert_eq!(stats.total_ridership, 0);
        assert_eq!(stats.total_revenue, 0);
        assert_eq!(stats.total_operating_cost, 0);
        assert_eq!(stats.mode_split_transit, 0.0);
    }

    // ─── Test 17: compute_fare_revenue zero ridership ───────────────────

    #[test]
    fn fare_revenue_zero_ridership() {
        assert_eq!(compute_fare_revenue(0, 250), 0);
    }

    // ─── Test 18: evaluate_routes empty lines ───────────────────────────

    #[test]
    fn evaluate_routes_empty() {
        let network = DefaultTransitNetwork::default();
        let metrics = network.evaluate_routes(&[]);
        assert!(metrics.is_empty());
    }
}
