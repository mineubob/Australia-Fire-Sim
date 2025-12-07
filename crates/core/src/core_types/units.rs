//! Semantic unit types for type-safe physical quantity handling
//!
//! This module provides newtype wrappers for physical quantities to prevent
//! accidental mixing of incompatible units (e.g., Celsius with Kelvin, or
//! meters with kilograms).
//!
//! # Design Philosophy
//! - Each type wraps f32 for performance (sufficient for fire simulation precision)
//! - Implements common traits (Add, Sub, Mul, Div, Ord, Display, etc.)
//! - Provides explicit conversion methods between related types
//! - Serde support for serialization
//! - Total ordering via Ord trait (NaN handled as greater than all values)
//!
//! # Usage
//! ```
//! use fire_sim_core::core_types::units::{Celsius, Kelvin, Meters};
//!
//! let temp = Celsius(25.0);
//! let kelvin: Kelvin = temp.into();
//! assert!((kelvin.0 - 298.15).abs() < 0.01);
//!
//! // Use standard min/max from Ord trait
//! let t1 = Celsius(100.0);
//! let t2 = Celsius(200.0);
//! assert_eq!(t1.min(t2), Celsius(100.0));
//! ```

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, AddAssign, Div, Mul, Sub, SubAssign};

// ============================================================================
// HELPER FUNCTION FOR TOTAL ORDERING OF F32
// ============================================================================

/// Compare f32 values with total ordering using Rust's built-in total_cmp
/// This is available since Rust 1.62 and handles NaN correctly
#[inline]
fn f32_total_cmp(a: f32, b: f32) -> Ordering {
    a.total_cmp(&b)
}

// ============================================================================
// TEMPERATURE TYPES
// ============================================================================

/// Temperature in degrees Celsius
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Celsius(pub f32);

impl Eq for Celsius {}

impl PartialOrd for Celsius {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Celsius {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl Celsius {
    /// Absolute zero in Celsius
    pub const ABSOLUTE_ZERO: Celsius = Celsius(-273.15);

    /// Water freezing point
    pub const FREEZING: Celsius = Celsius(0.0);

    /// Water boiling point at 1 atm
    pub const BOILING: Celsius = Celsius(100.0);

    /// Create a new Celsius temperature
    #[inline]
    pub fn new(value: f32) -> Self {
        Celsius(value)
    }

    /// Get the raw f32 value
    #[inline]
    pub fn value(self) -> f32 {
        self.0
    }

    /// Convert to Kelvin
    #[inline]
    pub fn to_kelvin(self) -> Kelvin {
        Kelvin(self.0 + 273.15)
    }
}

impl From<Celsius> for Kelvin {
    fn from(c: Celsius) -> Kelvin {
        c.to_kelvin()
    }
}

impl From<f32> for Celsius {
    fn from(v: f32) -> Self {
        Celsius(v)
    }
}

impl From<Celsius> for f32 {
    fn from(c: Celsius) -> f32 {
        c.0
    }
}

impl Add for Celsius {
    type Output = Celsius;
    fn add(self, rhs: Celsius) -> Celsius {
        Celsius(self.0 + rhs.0)
    }
}

impl Sub for Celsius {
    type Output = Celsius;
    fn sub(self, rhs: Celsius) -> Celsius {
        Celsius(self.0 - rhs.0)
    }
}

impl Mul<f32> for Celsius {
    type Output = Celsius;
    fn mul(self, rhs: f32) -> Celsius {
        Celsius(self.0 * rhs)
    }
}

impl Div<f32> for Celsius {
    type Output = Celsius;
    fn div(self, rhs: f32) -> Celsius {
        Celsius(self.0 / rhs)
    }
}

impl fmt::Display for Celsius {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1}°C", self.0)
    }
}

/// Temperature in Kelvin (absolute scale)
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Kelvin(pub f32);

impl Eq for Kelvin {}

impl PartialOrd for Kelvin {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Kelvin {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl Kelvin {
    /// Absolute zero
    pub const ABSOLUTE_ZERO: Kelvin = Kelvin(0.0);

    /// Convert to Celsius
    #[inline]
    pub fn to_celsius(self) -> Celsius {
        Celsius(self.0 - 273.15)
    }

    /// Get the raw f32 value
    #[inline]
    pub fn value(self) -> f32 {
        self.0
    }
}

impl From<Kelvin> for Celsius {
    fn from(k: Kelvin) -> Celsius {
        k.to_celsius()
    }
}

impl From<f32> for Kelvin {
    fn from(v: f32) -> Self {
        Kelvin(v)
    }
}

impl From<Kelvin> for f32 {
    fn from(k: Kelvin) -> f32 {
        k.0
    }
}

impl Add for Kelvin {
    type Output = Kelvin;
    fn add(self, rhs: Kelvin) -> Kelvin {
        Kelvin(self.0 + rhs.0)
    }
}

impl Sub for Kelvin {
    type Output = Kelvin;
    fn sub(self, rhs: Kelvin) -> Kelvin {
        Kelvin(self.0 - rhs.0)
    }
}

impl Mul<f32> for Kelvin {
    type Output = Kelvin;
    fn mul(self, rhs: f32) -> Kelvin {
        Kelvin(self.0 * rhs)
    }
}

impl Div<f32> for Kelvin {
    type Output = Kelvin;
    fn div(self, rhs: f32) -> Kelvin {
        Kelvin(self.0 / rhs)
    }
}

impl fmt::Display for Kelvin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1} K", self.0)
    }
}

// ============================================================================
// DISTANCE/LENGTH TYPES
// ============================================================================

/// Distance in meters
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Meters(pub f32);

impl Eq for Meters {}

impl PartialOrd for Meters {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Meters {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl Meters {
    /// Create a new distance in meters
    #[inline]
    pub fn new(value: f32) -> Self {
        Meters(value)
    }

    /// Get the raw f32 value
    #[inline]
    pub fn value(self) -> f32 {
        self.0
    }

    /// Convert to kilometers
    #[inline]
    pub fn to_kilometers(self) -> Kilometers {
        Kilometers(self.0 / 1000.0)
    }
}

impl From<f32> for Meters {
    fn from(v: f32) -> Self {
        Meters(v)
    }
}

impl From<Meters> for f32 {
    fn from(m: Meters) -> f32 {
        m.0
    }
}

impl Add for Meters {
    type Output = Meters;
    fn add(self, rhs: Meters) -> Meters {
        Meters(self.0 + rhs.0)
    }
}

impl Sub for Meters {
    type Output = Meters;
    fn sub(self, rhs: Meters) -> Meters {
        Meters(self.0 - rhs.0)
    }
}

impl Mul<f32> for Meters {
    type Output = Meters;
    fn mul(self, rhs: f32) -> Meters {
        Meters(self.0 * rhs)
    }
}

impl Div<f32> for Meters {
    type Output = Meters;
    fn div(self, rhs: f32) -> Meters {
        Meters(self.0 / rhs)
    }
}

// Cross-type operation: distance / time = velocity
impl Div<Seconds> for Meters {
    type Output = MetersPerSecond;
    fn div(self, rhs: Seconds) -> MetersPerSecond {
        MetersPerSecond(self.0 / rhs.0)
    }
}

impl fmt::Display for Meters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2} m", self.0)
    }
}

/// Distance in kilometers
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Kilometers(pub f32);

impl Eq for Kilometers {}

impl PartialOrd for Kilometers {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Kilometers {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl Kilometers {
    /// Convert to meters
    #[inline]
    pub fn to_meters(self) -> Meters {
        Meters(self.0 * 1000.0)
    }

    /// Get the raw f32 value
    #[inline]
    pub fn value(self) -> f32 {
        self.0
    }
}

impl From<Kilometers> for Meters {
    fn from(k: Kilometers) -> Meters {
        k.to_meters()
    }
}

impl Add for Kilometers {
    type Output = Kilometers;
    fn add(self, rhs: Kilometers) -> Kilometers {
        Kilometers(self.0 + rhs.0)
    }
}

impl Sub for Kilometers {
    type Output = Kilometers;
    fn sub(self, rhs: Kilometers) -> Kilometers {
        Kilometers(self.0 - rhs.0)
    }
}

impl Mul<f32> for Kilometers {
    type Output = Kilometers;
    fn mul(self, rhs: f32) -> Kilometers {
        Kilometers(self.0 * rhs)
    }
}

impl Div<f32> for Kilometers {
    type Output = Kilometers;
    fn div(self, rhs: f32) -> Kilometers {
        Kilometers(self.0 / rhs)
    }
}

// Cross-type operation: kilometers / hours = km/h
impl Div<Hours> for Kilometers {
    type Output = KilometersPerHour;
    fn div(self, rhs: Hours) -> KilometersPerHour {
        KilometersPerHour(self.0 / rhs.0)
    }
}

impl fmt::Display for Kilometers {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2} km", self.0)
    }
}

// ============================================================================
// MASS/DENSITY TYPES
// ============================================================================

/// Mass in kilograms
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Kilograms(pub f32);

impl Eq for Kilograms {}

impl PartialOrd for Kilograms {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Kilograms {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl Kilograms {
    /// Create a new mass in kilograms
    #[inline]
    pub fn new(value: f32) -> Self {
        Kilograms(value)
    }

    /// Get the raw f32 value
    #[inline]
    pub fn value(self) -> f32 {
        self.0
    }
}

impl From<f32> for Kilograms {
    fn from(v: f32) -> Self {
        Kilograms(v)
    }
}

impl fmt::Display for Kilograms {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2} kg", self.0)
    }
}

impl From<Kilograms> for f32 {
    fn from(k: Kilograms) -> f32 {
        k.0
    }
}

impl Add for Kilograms {
    type Output = Kilograms;
    fn add(self, rhs: Kilograms) -> Kilograms {
        Kilograms(self.0 + rhs.0)
    }
}

impl Sub for Kilograms {
    type Output = Kilograms;
    fn sub(self, rhs: Kilograms) -> Kilograms {
        Kilograms(self.0 - rhs.0)
    }
}

impl SubAssign<f32> for Kilograms {
    fn sub_assign(&mut self, rhs: f32) {
        self.0 -= rhs;
    }
}

impl AddAssign<f32> for Kilograms {
    fn add_assign(&mut self, rhs: f32) {
        self.0 += rhs;
    }
}

impl Mul<f32> for Kilograms {
    type Output = Kilograms;
    fn mul(self, rhs: f32) -> Kilograms {
        Kilograms(self.0 * rhs)
    }
}

impl Mul<Kilograms> for f32 {
    type Output = Kilograms;
    fn mul(self, rhs: Kilograms) -> Kilograms {
        Kilograms(self * rhs.0)
    }
}

impl Div<f32> for Kilograms {
    type Output = Kilograms;
    fn div(self, rhs: f32) -> Kilograms {
        Kilograms(self.0 / rhs)
    }
}

/// Density in kg/m³
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct KgPerCubicMeter(pub f32);

impl Eq for KgPerCubicMeter {}

impl PartialOrd for KgPerCubicMeter {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for KgPerCubicMeter {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl KgPerCubicMeter {
    /// Density of water at 4°C
    pub const WATER: KgPerCubicMeter = KgPerCubicMeter(1000.0);

    /// Density of air at sea level, 15°C
    pub const AIR_SEA_LEVEL: KgPerCubicMeter = KgPerCubicMeter(1.225);

    /// Create a new density
    #[inline]
    pub fn new(value: f32) -> Self {
        KgPerCubicMeter(value)
    }

    /// Get the raw f32 value
    #[inline]
    pub fn value(self) -> f32 {
        self.0
    }
}

impl From<f32> for KgPerCubicMeter {
    fn from(v: f32) -> Self {
        KgPerCubicMeter(v)
    }
}

impl From<KgPerCubicMeter> for f32 {
    fn from(d: KgPerCubicMeter) -> f32 {
        d.0
    }
}

impl fmt::Display for KgPerCubicMeter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2} kg/m³", self.0)
    }
}

// ============================================================================
// TIME TYPES
// ============================================================================

/// Time duration in seconds
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Seconds(pub f32);

impl Eq for Seconds {}

impl PartialOrd for Seconds {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Seconds {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl Seconds {
    /// Create a new duration in seconds
    #[inline]
    pub fn new(value: f32) -> Self {
        Seconds(value)
    }

    /// Get the raw f32 value
    #[inline]
    pub fn value(self) -> f32 {
        self.0
    }

    /// Convert to hours
    #[inline]
    pub fn to_hours(self) -> Hours {
        Hours(self.0 / 3600.0)
    }
}

impl From<f32> for Seconds {
    fn from(v: f32) -> Self {
        Seconds(v)
    }
}

impl From<Seconds> for f32 {
    fn from(s: Seconds) -> f32 {
        s.0
    }
}

impl Add for Seconds {
    type Output = Seconds;
    fn add(self, rhs: Seconds) -> Seconds {
        Seconds(self.0 + rhs.0)
    }
}

impl Sub for Seconds {
    type Output = Seconds;
    fn sub(self, rhs: Seconds) -> Seconds {
        Seconds(self.0 - rhs.0)
    }
}

impl Mul<f32> for Seconds {
    type Output = Seconds;
    fn mul(self, rhs: f32) -> Seconds {
        Seconds(self.0 * rhs)
    }
}

impl Div<f32> for Seconds {
    type Output = Seconds;
    fn div(self, rhs: f32) -> Seconds {
        Seconds(self.0 / rhs)
    }
}

impl fmt::Display for Seconds {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2} s", self.0)
    }
}

/// Time duration in hours
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Hours(pub f32);

impl Eq for Hours {}

impl PartialOrd for Hours {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Hours {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl Hours {
    /// Convert to seconds
    #[inline]
    pub fn to_seconds(self) -> Seconds {
        Seconds(self.0 * 3600.0)
    }

    /// Get the raw f32 value
    #[inline]
    pub fn value(self) -> f32 {
        self.0
    }
}

impl From<Hours> for Seconds {
    fn from(h: Hours) -> Seconds {
        h.to_seconds()
    }
}

impl From<f32> for Hours {
    fn from(v: f32) -> Self {
        Hours(v)
    }
}

impl fmt::Display for Hours {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2} h", self.0)
    }
}

// ============================================================================
// VELOCITY TYPES
// ============================================================================

/// Velocity in meters per second
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct MetersPerSecond(pub f32);

impl Eq for MetersPerSecond {}

impl PartialOrd for MetersPerSecond {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MetersPerSecond {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl MetersPerSecond {
    /// Create a new velocity
    #[inline]
    pub fn new(value: f32) -> Self {
        MetersPerSecond(value)
    }

    /// Get the raw f32 value
    #[inline]
    pub fn value(self) -> f32 {
        self.0
    }

    /// Convert to km/h
    #[inline]
    pub fn to_kmh(self) -> KilometersPerHour {
        KilometersPerHour(self.0 * 3.6)
    }
}

impl From<f32> for MetersPerSecond {
    fn from(v: f32) -> Self {
        MetersPerSecond(v)
    }
}

impl From<MetersPerSecond> for f32 {
    fn from(v: MetersPerSecond) -> f32 {
        v.0
    }
}

impl Add for MetersPerSecond {
    type Output = MetersPerSecond;
    fn add(self, rhs: MetersPerSecond) -> MetersPerSecond {
        MetersPerSecond(self.0 + rhs.0)
    }
}

impl Sub for MetersPerSecond {
    type Output = MetersPerSecond;
    fn sub(self, rhs: MetersPerSecond) -> MetersPerSecond {
        MetersPerSecond(self.0 - rhs.0)
    }
}

impl Mul<f32> for MetersPerSecond {
    type Output = MetersPerSecond;
    fn mul(self, rhs: f32) -> MetersPerSecond {
        MetersPerSecond(self.0 * rhs)
    }
}

impl Div<f32> for MetersPerSecond {
    type Output = MetersPerSecond;
    fn div(self, rhs: f32) -> MetersPerSecond {
        MetersPerSecond(self.0 / rhs)
    }
}

// Cross-type operation: velocity × time = distance
impl Mul<Seconds> for MetersPerSecond {
    type Output = Meters;
    fn mul(self, rhs: Seconds) -> Meters {
        Meters(self.0 * rhs.0)
    }
}

// Cross-type operation: time × velocity = distance
impl Mul<MetersPerSecond> for Seconds {
    type Output = Meters;
    fn mul(self, rhs: MetersPerSecond) -> Meters {
        Meters(self.0 * rhs.0)
    }
}

impl fmt::Display for MetersPerSecond {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2} m/s", self.0)
    }
}

/// Velocity in kilometers per hour
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct KilometersPerHour(pub f32);

impl Eq for KilometersPerHour {}

impl PartialOrd for KilometersPerHour {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for KilometersPerHour {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl KilometersPerHour {
    /// Convert to m/s
    #[inline]
    pub fn to_mps(self) -> MetersPerSecond {
        MetersPerSecond(self.0 / 3.6)
    }

    /// Get the raw f32 value
    #[inline]
    pub fn value(self) -> f32 {
        self.0
    }
}

impl From<KilometersPerHour> for MetersPerSecond {
    fn from(k: KilometersPerHour) -> MetersPerSecond {
        k.to_mps()
    }
}

impl From<f32> for KilometersPerHour {
    fn from(v: f32) -> Self {
        KilometersPerHour(v)
    }
}

impl Add for KilometersPerHour {
    type Output = KilometersPerHour;
    fn add(self, rhs: KilometersPerHour) -> KilometersPerHour {
        KilometersPerHour(self.0 + rhs.0)
    }
}

impl Sub for KilometersPerHour {
    type Output = KilometersPerHour;
    fn sub(self, rhs: KilometersPerHour) -> KilometersPerHour {
        KilometersPerHour(self.0 - rhs.0)
    }
}

impl Mul<f32> for KilometersPerHour {
    type Output = KilometersPerHour;
    fn mul(self, rhs: f32) -> KilometersPerHour {
        KilometersPerHour(self.0 * rhs)
    }
}

impl Div<f32> for KilometersPerHour {
    type Output = KilometersPerHour;
    fn div(self, rhs: f32) -> KilometersPerHour {
        KilometersPerHour(self.0 / rhs)
    }
}

// Cross-type operation: km/h × hours = kilometers
impl Mul<Hours> for KilometersPerHour {
    type Output = Kilometers;
    fn mul(self, rhs: Hours) -> Kilometers {
        Kilometers(self.0 * rhs.0)
    }
}

// Cross-type operation: hours × km/h = kilometers
impl Mul<KilometersPerHour> for Hours {
    type Output = Kilometers;
    fn mul(self, rhs: KilometersPerHour) -> Kilometers {
        Kilometers(self.0 * rhs.0)
    }
}

impl fmt::Display for KilometersPerHour {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1} km/h", self.0)
    }
}

// ============================================================================
// ENERGY/POWER TYPES
// ============================================================================

/// Energy in kilojoules
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Kilojoules(pub f32);

impl Eq for Kilojoules {}

impl PartialOrd for Kilojoules {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Kilojoules {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl Kilojoules {
    /// Create a new energy value
    #[inline]
    pub fn new(value: f32) -> Self {
        Kilojoules(value)
    }

    /// Get the raw f32 value
    #[inline]
    pub fn value(self) -> f32 {
        self.0
    }
}

impl From<f32> for Kilojoules {
    fn from(v: f32) -> Self {
        Kilojoules(v)
    }
}

impl From<Kilojoules> for f32 {
    fn from(kj: Kilojoules) -> f32 {
        kj.0
    }
}

impl Add for Kilojoules {
    type Output = Kilojoules;
    fn add(self, rhs: Kilojoules) -> Kilojoules {
        Kilojoules(self.0 + rhs.0)
    }
}

impl Sub for Kilojoules {
    type Output = Kilojoules;
    fn sub(self, rhs: Kilojoules) -> Kilojoules {
        Kilojoules(self.0 - rhs.0)
    }
}

impl Mul<f32> for Kilojoules {
    type Output = Kilojoules;
    fn mul(self, rhs: f32) -> Kilojoules {
        Kilojoules(self.0 * rhs)
    }
}

impl Div<f32> for Kilojoules {
    type Output = Kilojoules;
    fn div(self, rhs: f32) -> Kilojoules {
        Kilojoules(self.0 / rhs)
    }
}

impl fmt::Display for Kilojoules {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2} kJ", self.0)
    }
}

/// Heat content in kJ/kg
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct KjPerKg(pub f32);

impl Eq for KjPerKg {}

impl PartialOrd for KjPerKg {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for KjPerKg {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl KjPerKg {
    /// Latent heat of vaporization for water (2260 kJ/kg)
    pub const WATER_LATENT_HEAT: KjPerKg = KjPerKg(2260.0);

    /// Create a new heat content value
    #[inline]
    pub fn new(value: f32) -> Self {
        KjPerKg(value)
    }

    /// Get the raw f32 value
    #[inline]
    pub fn value(self) -> f32 {
        self.0
    }
}

impl From<f32> for KjPerKg {
    fn from(v: f32) -> Self {
        KjPerKg(v)
    }
}

impl From<KjPerKg> for f32 {
    fn from(h: KjPerKg) -> f32 {
        h.0
    }
}

impl Mul<f32> for KjPerKg {
    type Output = KjPerKg;
    fn mul(self, rhs: f32) -> KjPerKg {
        KjPerKg(self.0 * rhs)
    }
}

impl Mul<KjPerKg> for f32 {
    type Output = KjPerKg;
    fn mul(self, rhs: KjPerKg) -> KjPerKg {
        KjPerKg(self * rhs.0)
    }
}

// Cross-type operation: kJ/kg × kg = kJ
impl Mul<Kilograms> for KjPerKg {
    type Output = Kilojoules;
    fn mul(self, rhs: Kilograms) -> Kilojoules {
        Kilojoules(self.0 * rhs.0)
    }
}

// Cross-type operation: kg × kJ/kg = kJ
impl Mul<KjPerKg> for Kilograms {
    type Output = Kilojoules;
    fn mul(self, rhs: KjPerKg) -> Kilojoules {
        Kilojoules(self.0 * rhs.0)
    }
}

impl fmt::Display for KjPerKg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1} kJ/kg", self.0)
    }
}

/// Fire intensity in kW/m (Byram's fireline intensity)
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct KwPerMeter(pub f32);

impl Eq for KwPerMeter {}

impl PartialOrd for KwPerMeter {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for KwPerMeter {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl KwPerMeter {
    /// Create a new fire intensity value
    #[inline]
    pub fn new(value: f32) -> Self {
        KwPerMeter(value)
    }

    /// Get the raw f32 value
    #[inline]
    pub fn value(self) -> f32 {
        self.0
    }

    /// Low intensity fire threshold (< 350 kW/m)
    pub const LOW_INTENSITY_THRESHOLD: KwPerMeter = KwPerMeter(350.0);

    /// Moderate intensity threshold (350-2000 kW/m)
    pub const MODERATE_INTENSITY_THRESHOLD: KwPerMeter = KwPerMeter(2000.0);

    /// High intensity threshold (2000-4000 kW/m)
    pub const HIGH_INTENSITY_THRESHOLD: KwPerMeter = KwPerMeter(4000.0);

    /// Extreme intensity (> 10000 kW/m, crown fire conditions)
    pub const EXTREME_INTENSITY_THRESHOLD: KwPerMeter = KwPerMeter(10000.0);
}

impl From<f32> for KwPerMeter {
    fn from(v: f32) -> Self {
        KwPerMeter(v)
    }
}

impl From<KwPerMeter> for f32 {
    fn from(i: KwPerMeter) -> f32 {
        i.0
    }
}

impl fmt::Display for KwPerMeter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1} kW/m", self.0)
    }
}

// ============================================================================
// SPECIFIC HEAT TYPE
// ============================================================================

/// Specific heat capacity in kJ/(kg·K)
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct KjPerKgK(pub f32);

impl Eq for KjPerKgK {}

impl PartialOrd for KjPerKgK {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for KjPerKgK {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl KjPerKgK {
    /// Specific heat of water (4.18 kJ/(kg·K))
    pub const WATER: KjPerKgK = KjPerKgK(4.18);

    /// Specific heat of dry wood (approximately 1.5 kJ/(kg·K))
    pub const DRY_WOOD: KjPerKgK = KjPerKgK(1.5);

    /// Specific heat of air at constant pressure (1.005 kJ/(kg·K))
    pub const AIR: KjPerKgK = KjPerKgK(1.005);

    /// Create a new specific heat value
    #[inline]
    pub fn new(value: f32) -> Self {
        KjPerKgK(value)
    }

    /// Get the raw f32 value
    #[inline]
    pub fn value(self) -> f32 {
        self.0
    }
}

impl From<f32> for KjPerKgK {
    fn from(v: f32) -> Self {
        KjPerKgK(v)
    }
}

impl From<KjPerKgK> for f32 {
    fn from(c: KjPerKgK) -> f32 {
        c.0
    }
}

impl Mul<f32> for KjPerKgK {
    type Output = KjPerKgK;
    fn mul(self, rhs: f32) -> KjPerKgK {
        KjPerKgK(self.0 * rhs)
    }
}

impl Mul<KjPerKgK> for f32 {
    type Output = KjPerKgK;
    fn mul(self, rhs: KjPerKgK) -> KjPerKgK {
        KjPerKgK(self * rhs.0)
    }
}

impl fmt::Display for KjPerKgK {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.3} kJ/(kg·K)", self.0)
    }
}

// ============================================================================
// FRACTION/RATIO TYPES
// ============================================================================

/// A fraction in the range [0, 1]
/// Represents moisture content, efficiency ratios, damping coefficients, etc.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Fraction(pub f32);

impl Eq for Fraction {}

impl PartialOrd for Fraction {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Fraction {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl Fraction {
    /// Zero fraction
    pub const ZERO: Fraction = Fraction(0.0);

    /// Full/complete (1.0)
    pub const ONE: Fraction = Fraction(1.0);

    /// Create a new fraction, clamping to [0, 1]
    #[inline]
    pub fn new(value: f32) -> Self {
        Fraction(value.clamp(0.0, 1.0))
    }

    /// Create a fraction without clamping (for performance when value is known valid)
    #[inline]
    pub fn new_unchecked(value: f32) -> Self {
        Fraction(value)
    }

    /// Get the raw f32 value
    #[inline]
    pub fn value(self) -> f32 {
        self.0
    }

    /// Convert to percentage (0-100)
    #[inline]
    pub fn to_percent(self) -> Percent {
        Percent(self.0 * 100.0)
    }
}

impl fmt::Display for Fraction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.4}", self.0)
    }
}

impl From<f32> for Fraction {
    fn from(v: f32) -> Self {
        Fraction::new(v)
    }
}

impl From<Fraction> for f32 {
    fn from(f: Fraction) -> f32 {
        f.0
    }
}

impl Add for Fraction {
    type Output = Fraction;
    fn add(self, rhs: Fraction) -> Fraction {
        Fraction::new(self.0 + rhs.0)
    }
}

impl Sub for Fraction {
    type Output = Fraction;
    fn sub(self, rhs: Fraction) -> Fraction {
        Fraction::new(self.0 - rhs.0)
    }
}

impl Mul<Fraction> for Fraction {
    type Output = Fraction;
    fn mul(self, rhs: Fraction) -> Fraction {
        Fraction::new(self.0 * rhs.0)
    }
}

impl Div<Fraction> for Fraction {
    type Output = f32;
    fn div(self, rhs: Fraction) -> f32 {
        self.0 / rhs.0
    }
}

impl Mul<f32> for Fraction {
    type Output = f32;
    fn mul(self, rhs: f32) -> f32 {
        self.0 * rhs
    }
}

impl Mul<Fraction> for f32 {
    type Output = f32;
    fn mul(self, rhs: Fraction) -> f32 {
        self * rhs.0
    }
}

/// A percentage (0-100)
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Percent(pub f32);

impl Eq for Percent {}

impl PartialOrd for Percent {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Percent {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl Percent {
    /// Create a new percentage
    #[inline]
    pub fn new(value: f32) -> Self {
        Percent(value)
    }

    /// Get the raw f32 value
    #[inline]
    pub fn value(self) -> f32 {
        self.0
    }

    /// Convert to fraction (0-1)
    #[inline]
    pub fn to_fraction(self) -> Fraction {
        Fraction(self.0 / 100.0)
    }
}

impl From<f32> for Percent {
    fn from(v: f32) -> Self {
        Percent(v)
    }
}

impl From<Percent> for f32 {
    fn from(p: Percent) -> f32 {
        p.0
    }
}

impl From<Percent> for Fraction {
    fn from(p: Percent) -> Fraction {
        p.to_fraction()
    }
}

impl From<Fraction> for Percent {
    fn from(f: Fraction) -> Percent {
        f.to_percent()
    }
}

impl Div<f32> for Percent {
    type Output = Percent;
    fn div(self, rhs: f32) -> Percent {
        Percent(self.0 / rhs)
    }
}

impl Mul<f32> for Percent {
    type Output = Percent;
    fn mul(self, rhs: f32) -> Percent {
        Percent(self.0 * rhs)
    }
}

impl Add for Percent {
    type Output = Percent;
    fn add(self, rhs: Percent) -> Percent {
        Percent(self.0 + rhs.0)
    }
}

impl Sub for Percent {
    type Output = Percent;
    fn sub(self, rhs: Percent) -> Percent {
        Percent(self.0 - rhs.0)
    }
}

impl fmt::Display for Percent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1}%", self.0)
    }
}

// ============================================================================
// ANGLE TYPES
// ============================================================================

/// Angle in degrees
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Degrees(pub f32);

impl Eq for Degrees {}

impl PartialOrd for Degrees {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Degrees {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl Degrees {
    /// Create a new angle in degrees
    #[inline]
    pub fn new(value: f32) -> Self {
        Degrees(value)
    }

    /// Get the raw f32 value
    #[inline]
    pub fn value(self) -> f32 {
        self.0
    }

    /// Convert to radians
    #[inline]
    pub fn to_radians(self) -> Radians {
        Radians(self.0.to_radians())
    }
}

impl From<f32> for Degrees {
    fn from(v: f32) -> Self {
        Degrees(v)
    }
}

impl From<Degrees> for f32 {
    fn from(d: Degrees) -> f32 {
        d.0
    }
}

impl From<Degrees> for Radians {
    fn from(d: Degrees) -> Radians {
        d.to_radians()
    }
}

impl fmt::Display for Degrees {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1}°", self.0)
    }
}

/// Angle in radians
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Radians(pub f32);

impl Eq for Radians {}

impl PartialOrd for Radians {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Radians {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl Radians {
    /// Create a new angle in radians
    #[inline]
    pub fn new(value: f32) -> Self {
        Radians(value)
    }

    /// Get the raw f32 value
    #[inline]
    pub fn value(self) -> f32 {
        self.0
    }

    /// Convert to degrees
    #[inline]
    pub fn to_degrees(self) -> Degrees {
        Degrees(self.0.to_degrees())
    }

    /// Compute sine
    #[inline]
    pub fn sin(self) -> f32 {
        self.0.sin()
    }

    /// Compute cosine
    #[inline]
    pub fn cos(self) -> f32 {
        self.0.cos()
    }

    /// Compute tangent
    #[inline]
    pub fn tan(self) -> f32 {
        self.0.tan()
    }
}

impl From<f32> for Radians {
    fn from(v: f32) -> Self {
        Radians(v)
    }
}

impl From<Radians> for f32 {
    fn from(r: Radians) -> f32 {
        r.0
    }
}

impl From<Radians> for Degrees {
    fn from(r: Radians) -> Degrees {
        r.to_degrees()
    }
}

impl fmt::Display for Radians {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.4} rad", self.0)
    }
}

// ============================================================================
// SURFACE AREA TO VOLUME RATIO
// ============================================================================

/// Surface area to volume ratio in m²/m³
/// Critical for fire spread calculations (Rothermel model)
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct SurfaceAreaToVolume(pub f32);

impl Eq for SurfaceAreaToVolume {}

impl PartialOrd for SurfaceAreaToVolume {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SurfaceAreaToVolume {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl SurfaceAreaToVolume {
    /// Fine fuels (grass, leaves): 3000-4000 m²/m³
    pub const FINE_FUELS: SurfaceAreaToVolume = SurfaceAreaToVolume(3500.0);

    /// Medium fuels (twigs): 500-1000 m²/m³
    pub const MEDIUM_FUELS: SurfaceAreaToVolume = SurfaceAreaToVolume(750.0);

    /// Coarse fuels (branches): 100-300 m²/m³
    pub const COARSE_FUELS: SurfaceAreaToVolume = SurfaceAreaToVolume(200.0);

    /// Stringybark (fibrous bark strips): 50-200 m²/m³
    pub const STRINGYBARK: SurfaceAreaToVolume = SurfaceAreaToVolume(150.0);

    /// Smooth bark: 50-100 m²/m³
    pub const SMOOTH_BARK: SurfaceAreaToVolume = SurfaceAreaToVolume(80.0);

    /// Create a new surface area to volume ratio
    #[inline]
    pub fn new(value: f32) -> Self {
        SurfaceAreaToVolume(value)
    }

    /// Get the raw f32 value
    #[inline]
    pub fn value(self) -> f32 {
        self.0
    }
}

impl From<f32> for SurfaceAreaToVolume {
    fn from(v: f32) -> Self {
        SurfaceAreaToVolume(v)
    }
}

impl From<SurfaceAreaToVolume> for f32 {
    fn from(s: SurfaceAreaToVolume) -> f32 {
        s.0
    }
}

impl Div<f32> for SurfaceAreaToVolume {
    type Output = f32;
    fn div(self, rhs: f32) -> f32 {
        self.0 / rhs
    }
}

impl fmt::Display for SurfaceAreaToVolume {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.0} m²/m³", self.0)
    }
}

// ============================================================================
// TESTS
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_celsius_to_kelvin() {
        let c = Celsius(25.0);
        let k = c.to_kelvin();
        assert!((k.0 - 298.15).abs() < 0.01);
    }

    #[test]
    fn test_kelvin_to_celsius() {
        let k = Kelvin(273.15);
        let c = k.to_celsius();
        assert!((c.0 - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_meters_to_kilometers() {
        let m = Meters(5000.0);
        let km = m.to_kilometers();
        assert!((km.0 - 5.0).abs() < 0.01);
    }

    #[test]
    fn test_mps_to_kmh() {
        let mps = MetersPerSecond(10.0);
        let kmh = mps.to_kmh();
        assert!((kmh.0 - 36.0).abs() < 0.01);
    }

    #[test]
    fn test_fraction_clamping() {
        let f1 = Fraction::new(1.5);
        assert_eq!(f1.0, 1.0);

        let f2 = Fraction::new(-0.5);
        assert_eq!(f2.0, 0.0);

        let f3 = Fraction::new(0.5);
        assert_eq!(f3.0, 0.5);
    }

    #[test]
    fn test_fraction_to_percent() {
        let f = Fraction(0.75);
        let p = f.to_percent();
        assert!((p.0 - 75.0).abs() < 0.01);
    }

    #[test]
    fn test_degrees_to_radians() {
        let d = Degrees(180.0);
        let r = d.to_radians();
        assert!((r.0 - std::f32::consts::PI).abs() < 0.01);
    }

    #[test]
    fn test_seconds_to_hours() {
        let s = Seconds(3600.0);
        let h = s.to_hours();
        assert!((h.0 - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_velocity_times_time_equals_distance() {
        let velocity = MetersPerSecond(10.0);
        let time = Seconds(5.0);
        let distance: Meters = velocity * time;
        assert!((distance.0 - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_time_times_velocity_equals_distance() {
        let time = Seconds(5.0);
        let velocity = MetersPerSecond(10.0);
        let distance: Meters = time * velocity;
        assert!((distance.0 - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_distance_divided_by_time_equals_velocity() {
        let distance = Meters(100.0);
        let time = Seconds(10.0);
        let velocity: MetersPerSecond = distance / time;
        assert!((velocity.0 - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_kmh_times_hours_equals_km() {
        let speed = KilometersPerHour(60.0);
        let time = Hours(2.0);
        let distance: Kilometers = speed * time;
        assert!((distance.0 - 120.0).abs() < 0.01);
    }

    #[test]
    fn test_km_divided_by_hours_equals_kmh() {
        let distance = Kilometers(120.0);
        let time = Hours(2.0);
        let speed: KilometersPerHour = distance / time;
        assert!((speed.0 - 60.0).abs() < 0.01);
    }

    #[test]
    fn test_heat_content_times_mass_equals_energy() {
        let heat = KjPerKg(2260.0);
        let mass = Kilograms(0.5);
        let energy: Kilojoules = heat * mass;
        assert!((energy.0 - 1130.0).abs() < 0.01);
    }

    #[test]
    fn test_fraction_arithmetic() {
        let a = Fraction::new(0.5);
        let b = Fraction::new(0.3);
        
        let sum = a + b;
        assert!((sum.0 - 0.8).abs() < 0.01);
        
        let diff = a - b;
        assert!((diff.0 - 0.2).abs() < 0.01);
        
        let prod = a * b;
        assert!((prod.0 - 0.15).abs() < 0.01);
    }
}
