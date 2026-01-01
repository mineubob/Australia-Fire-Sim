//! Semantic unit types for type-safe physical quantity handling
//!
//! This module provides newtype wrappers for physical quantities to prevent
//! accidental mixing of incompatible units (e.g., Celsius with Kelvin, or
//! meters with kilograms).
//!
//! # Design Philosophy
//! - Temperature types use f64 for high-precision T^4 calculations (Stefan-Boltzmann)
//! - Spatial/Mass/Angle types use f32 for performance where precision is adequate
//! - Implements common traits (Add, Sub, Mul, Div, Ord, Display, etc.)
//! - Provides explicit conversion methods between related types
//! - Serde support for serialization
//! - Total ordering via Ord trait (NaN handled as greater than all values)
//! - Private inner fields with validated constructors
//!
//! # Usage
//! ```
//! use fire_sim_core::core_types::units::{Celsius, Kelvin, Meters};
//!
//! let temp = Celsius::new(25.0);
//! let kelvin: Kelvin = temp.into();
//! assert!((*kelvin - 298.15).abs() < 0.01);
//!
//! // Use standard min/max from Ord trait
//! let t1 = Celsius::new(100.0);
//! let t2 = Celsius::new(200.0);
//! assert_eq!(t1.min(t2), Celsius::new(100.0));
//! ```

use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt;
use std::ops::{Add, AddAssign, Deref, DerefMut, Div, Mul, Neg, Sub, SubAssign};

// ============================================================================
// HELPER FUNCTIONS FOR TOTAL ORDERING
// ============================================================================

/// Compare f32 values with total ordering using Rust's built-in `total_cmp`
/// This is available since Rust 1.62 and handles NaN correctly
#[inline]
fn f32_total_cmp(a: f32, b: f32) -> Ordering {
    a.total_cmp(&b)
}

/// Compare f64 values with total ordering using Rust's built-in `total_cmp`
/// Used for high-precision temperature types
#[inline]
fn f64_total_cmp(a: f64, b: f64) -> Ordering {
    a.total_cmp(&b)
}

// ============================================================================
// TEMPERATURE TYPES (f64 for high-precision T^4 calculations)
// ============================================================================

/// Temperature in degrees Celsius
/// Uses f64 for Stefan-Boltzmann law (T^4) calculations
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Celsius(f64);

impl Eq for Celsius {}

impl PartialOrd for Celsius {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Celsius {
    fn cmp(&self, other: &Self) -> Ordering {
        f64_total_cmp(self.0, other.0)
    }
}

impl Deref for Celsius {
    type Target = f64;
    #[inline]
    fn deref(&self) -> &f64 {
        &self.0
    }
}

impl DerefMut for Celsius {
    #[inline]
    fn deref_mut(&mut self) -> &mut f64 {
        &mut self.0
    }
}

impl Celsius {
    /// Absolute zero in Celsius
    pub const ABSOLUTE_ZERO: Celsius = Celsius(-273.15);

    /// Celsius to Kelvin conversion offset (0°C = 273.15 K)
    const CELSIUS_KELVIN_OFFSET: f64 = 273.15;

    /// Water freezing point
    pub const FREEZING: Celsius = Celsius(0.0);

    /// Water boiling point at 1 atm
    pub const BOILING: Celsius = Celsius(100.0);

    /// Create a new Celsius temperature. Asserts value >= absolute zero (-273.15°C).
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn new(value: f64) -> Self {
        assert!(
            value >= -Self::CELSIUS_KELVIN_OFFSET,
            "Celsius::new: value is below absolute zero (-273.15°C)"
        );
        Celsius(value)
    }

    /// Create without validation.
    /// # Safety
    /// Caller must ensure value >= -273.15 (absolute zero).
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(value: f64) -> Self {
        Celsius(value)
    }

    /// Convert to Kelvin
    #[inline]
    #[must_use]
    pub fn to_kelvin(self) -> Kelvin {
        Kelvin(self.0 + Self::CELSIUS_KELVIN_OFFSET)
    }

    /// Convert to f32 for calculations requiring lower precision
    #[inline]
    #[must_use]
    pub fn as_f32(self) -> f32 {
        self.0 as f32
    }
}

impl From<Celsius> for Kelvin {
    fn from(c: Celsius) -> Kelvin {
        c.to_kelvin()
    }
}

impl From<f64> for Celsius {
    fn from(v: f64) -> Self {
        Celsius(v)
    }
}

impl From<f32> for Celsius {
    fn from(v: f32) -> Self {
        Celsius(f64::from(v))
    }
}

impl From<Celsius> for f64 {
    fn from(c: Celsius) -> f64 {
        c.0
    }
}

/// Temperature difference/delta in Celsius
/// Can be any value (positive or negative)
/// Used for temperature changes, differences, and relative values
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct CelsiusDelta(f64);

impl Eq for CelsiusDelta {}

impl PartialOrd for CelsiusDelta {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CelsiusDelta {
    fn cmp(&self, other: &Self) -> Ordering {
        f64_total_cmp(self.0, other.0)
    }
}

impl CelsiusDelta {
    /// Create a temperature delta (can be any value, positive or negative)
    #[inline]
    #[must_use]
    pub const fn new(value: f64) -> Self {
        CelsiusDelta(value)
    }

    /// Get the raw f64 value
    #[inline]
    #[must_use]
    pub fn value(self) -> f64 {
        self.0
    }

    /// Convert to f32 (for compatibility with legacy code)
    #[inline]
    #[must_use]
    pub fn as_f32(self) -> f32 {
        self.0 as f32
    }

    /// Absolute value of the delta
    #[inline]
    #[must_use]
    pub fn abs(self) -> Self {
        CelsiusDelta(self.0.abs())
    }
}

impl Deref for CelsiusDelta {
    type Target = f64;
    #[inline]
    fn deref(&self) -> &f64 {
        &self.0
    }
}

impl Neg for CelsiusDelta {
    type Output = CelsiusDelta;
    fn neg(self) -> CelsiusDelta {
        CelsiusDelta(-self.0)
    }
}

impl Add<CelsiusDelta> for CelsiusDelta {
    type Output = CelsiusDelta;
    fn add(self, rhs: CelsiusDelta) -> CelsiusDelta {
        CelsiusDelta(self.0 + rhs.0)
    }
}

impl Sub<CelsiusDelta> for CelsiusDelta {
    type Output = CelsiusDelta;
    fn sub(self, rhs: CelsiusDelta) -> CelsiusDelta {
        CelsiusDelta(self.0 - rhs.0)
    }
}

impl Mul<f64> for CelsiusDelta {
    type Output = CelsiusDelta;
    fn mul(self, rhs: f64) -> CelsiusDelta {
        CelsiusDelta(self.0 * rhs)
    }
}

impl Div<f64> for CelsiusDelta {
    type Output = CelsiusDelta;
    fn div(self, rhs: f64) -> CelsiusDelta {
        CelsiusDelta(self.0 / rhs)
    }
}

impl PartialEq<f64> for CelsiusDelta {
    fn eq(&self, other: &f64) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<f64> for CelsiusDelta {
    fn partial_cmp(&self, other: &f64) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

impl fmt::Display for CelsiusDelta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1}°C", self.0)
    }
}

// Celsius + CelsiusDelta = Celsius (adding a change to absolute temperature)
impl Add<CelsiusDelta> for Celsius {
    type Output = Celsius;
    fn add(self, rhs: CelsiusDelta) -> Celsius {
        let result = self.0 + rhs.0;
        assert!(
            result >= *Celsius::ABSOLUTE_ZERO,
            "Temperature below absolute zero: {result:.2}°C"
        );
        Celsius(result)
    }
}

// Celsius - CelsiusDelta = Celsius (subtracting a change from absolute temperature)
impl Sub<CelsiusDelta> for Celsius {
    type Output = Celsius;
    fn sub(self, rhs: CelsiusDelta) -> Celsius {
        let result = self.0 - rhs.0;
        assert!(
            result >= *Celsius::ABSOLUTE_ZERO,
            "Temperature below absolute zero: {result:.2}°C"
        );
        Celsius(result)
    }
}

// Celsius - Celsius = CelsiusDelta (difference between two absolute temperatures)
impl Sub for Celsius {
    type Output = CelsiusDelta;
    fn sub(self, rhs: Celsius) -> CelsiusDelta {
        // Result is a delta - can be any value
        CelsiusDelta(self.0 - rhs.0)
    }
}

// Celsius * f64 = Celsius (scaling absolute temperature, e.g., interpolation)
impl Mul<f64> for Celsius {
    type Output = Celsius;
    fn mul(self, rhs: f64) -> Celsius {
        let result = self.0 * rhs;
        assert!(
            result >= *Celsius::ABSOLUTE_ZERO,
            "Temperature below absolute zero: {result:.2}°C"
        );
        Celsius(result)
    }
}

// Celsius / f64 = Celsius (dividing absolute temperature)
impl Div<f64> for Celsius {
    type Output = Celsius;
    fn div(self, rhs: f64) -> Celsius {
        let result = self.0 / rhs;
        assert!(
            result >= *Celsius::ABSOLUTE_ZERO,
            "Temperature below absolute zero: {result:.2}°C"
        );
        Celsius(result)
    }
}

impl PartialEq<f64> for Celsius {
    fn eq(&self, other: &f64) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<f64> for Celsius {
    fn partial_cmp(&self, other: &f64) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

impl fmt::Display for Celsius {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1}°C", self.0)
    }
}

/// Temperature in Kelvin (absolute scale)
/// Uses f64 for Stefan-Boltzmann law (T^4) calculations
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Kelvin(f64);

impl Eq for Kelvin {}

impl PartialOrd for Kelvin {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Kelvin {
    fn cmp(&self, other: &Self) -> Ordering {
        f64_total_cmp(self.0, other.0)
    }
}

impl Deref for Kelvin {
    type Target = f64;
    #[inline]
    fn deref(&self) -> &f64 {
        &self.0
    }
}

impl DerefMut for Kelvin {
    #[inline]
    fn deref_mut(&mut self) -> &mut f64 {
        &mut self.0
    }
}

impl Kelvin {
    /// Absolute zero
    pub const ABSOLUTE_ZERO: Kelvin = Kelvin(0.0);

    /// Create a new Kelvin temperature. Asserts value >= absolute zero (0 K).
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn new(value: f64) -> Self {
        assert!(
            value >= 0.0,
            "Kelvin::new: value is below absolute zero (0 K)"
        );
        Kelvin(value)
    }

    /// Create without validation.
    /// # Safety
    /// Caller must ensure value >= 0 (absolute zero).
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(value: f64) -> Self {
        Kelvin(value)
    }

    /// Convert to Celsius
    #[inline]
    #[must_use]
    pub fn to_celsius(self) -> Celsius {
        Celsius::new(self.0 - Celsius::CELSIUS_KELVIN_OFFSET)
    }

    /// Convert to f32 for calculations requiring lower precision
    #[inline]
    #[must_use]
    pub fn as_f32(self) -> f32 {
        self.0 as f32
    }
}

impl From<Kelvin> for Celsius {
    fn from(k: Kelvin) -> Celsius {
        k.to_celsius()
    }
}

impl From<f64> for Kelvin {
    fn from(v: f64) -> Self {
        Kelvin::new(v)
    }
}

impl From<f32> for Kelvin {
    fn from(v: f32) -> Self {
        Kelvin::new(f64::from(v))
    }
}

impl From<Kelvin> for f64 {
    fn from(k: Kelvin) -> f64 {
        k.0
    }
}

/// Temperature difference/delta in Kelvin
/// Can be any value (positive or negative)
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct KelvinDelta(f64);

impl Eq for KelvinDelta {}

impl PartialOrd for KelvinDelta {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for KelvinDelta {
    fn cmp(&self, other: &Self) -> Ordering {
        f64_total_cmp(self.0, other.0)
    }
}

impl KelvinDelta {
    /// Create a temperature delta (can be any value)
    #[inline]
    #[must_use]
    pub const fn new(value: f64) -> Self {
        KelvinDelta(value)
    }

    /// Get the raw f64 value
    #[inline]
    #[must_use]
    pub fn value(self) -> f64 {
        self.0
    }

    /// Absolute value of the delta
    #[inline]
    #[must_use]
    pub fn abs(self) -> Self {
        KelvinDelta(self.0.abs())
    }
}

impl Deref for KelvinDelta {
    type Target = f64;
    #[inline]
    fn deref(&self) -> &f64 {
        &self.0
    }
}

impl Neg for KelvinDelta {
    type Output = KelvinDelta;
    fn neg(self) -> KelvinDelta {
        KelvinDelta(-self.0)
    }
}

impl Add<KelvinDelta> for KelvinDelta {
    type Output = KelvinDelta;
    fn add(self, rhs: KelvinDelta) -> KelvinDelta {
        KelvinDelta(self.0 + rhs.0)
    }
}

impl Sub<KelvinDelta> for KelvinDelta {
    type Output = KelvinDelta;
    fn sub(self, rhs: KelvinDelta) -> KelvinDelta {
        KelvinDelta(self.0 - rhs.0)
    }
}

impl Mul<f64> for KelvinDelta {
    type Output = KelvinDelta;
    fn mul(self, rhs: f64) -> KelvinDelta {
        KelvinDelta(self.0 * rhs)
    }
}

impl Div<f64> for KelvinDelta {
    type Output = KelvinDelta;
    fn div(self, rhs: f64) -> KelvinDelta {
        KelvinDelta(self.0 / rhs)
    }
}

impl fmt::Display for KelvinDelta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.1} K", self.0)
    }
}

// Kelvin + KelvinDelta = Kelvin
impl Add<KelvinDelta> for Kelvin {
    type Output = Kelvin;
    fn add(self, rhs: KelvinDelta) -> Kelvin {
        let result = self.0 + rhs.0;
        assert!(
            result >= *Kelvin::ABSOLUTE_ZERO,
            "Temperature below absolute zero: {result:.2} K"
        );
        Kelvin(result)
    }
}

// Kelvin - KelvinDelta = Kelvin
impl Sub<KelvinDelta> for Kelvin {
    type Output = Kelvin;
    fn sub(self, rhs: KelvinDelta) -> Kelvin {
        let result = self.0 - rhs.0;
        assert!(
            result >= *Kelvin::ABSOLUTE_ZERO,
            "Temperature below absolute zero: {result:.2} K"
        );
        Kelvin(result)
    }
}

// Kelvin - Kelvin = KelvinDelta
impl Sub for Kelvin {
    type Output = KelvinDelta;
    fn sub(self, rhs: Kelvin) -> KelvinDelta {
        KelvinDelta(self.0 - rhs.0)
    }
}

// Kelvin * f64 = Kelvin (for interpolation, etc.)
impl Mul<f64> for Kelvin {
    type Output = Kelvin;
    fn mul(self, rhs: f64) -> Kelvin {
        let result = self.0 * rhs;
        assert!(
            result >= *Kelvin::ABSOLUTE_ZERO,
            "Temperature below absolute zero: {result:.2} K"
        );
        Kelvin(result)
    }
}

// Kelvin / f64 = Kelvin
impl Div<f64> for Kelvin {
    type Output = Kelvin;
    fn div(self, rhs: f64) -> Kelvin {
        let result = self.0 / rhs;
        assert!(
            result >= *Kelvin::ABSOLUTE_ZERO,
            "Temperature below absolute zero: {result:.2} K"
        );
        Kelvin(result)
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
pub struct Meters(f32);

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

impl Deref for Meters {
    type Target = f32;
    #[inline]
    fn deref(&self) -> &f32 {
        &self.0
    }
}

impl DerefMut for Meters {
    #[inline]
    fn deref_mut(&mut self) -> &mut f32 {
        &mut self.0
    }
}

impl Meters {
    /// Create a new distance in meters
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn new(value: f32) -> Self {
        assert!(value >= 0.0, "Meters::new: negative distance is invalid");
        Meters(value)
    }

    /// Create without validation.
    /// # Safety
    /// Caller must ensure value >= 0 (non-negative distance).
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(value: f32) -> Self {
        Meters(value)
    }

    /// Get the raw f32 value
    #[inline]
    #[must_use]
    pub fn value(self) -> f32 {
        self.0
    }

    /// Convert to kilometers
    #[inline]
    #[must_use]
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
pub struct Kilometers(f32);

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

impl Deref for Kilometers {
    type Target = f32;
    #[inline]
    fn deref(&self) -> &f32 {
        &self.0
    }
}

impl DerefMut for Kilometers {
    #[inline]
    fn deref_mut(&mut self) -> &mut f32 {
        &mut self.0
    }
}

impl Kilometers {
    /// Create a new Kilometers value.
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn new(value: f32) -> Self {
        assert!(
            value >= 0.0,
            "Kilometers::new: negative distance is invalid"
        );
        Kilometers(value)
    }

    /// Create without validation.
    /// # Safety
    /// Caller must ensure value >= 0 (non-negative distance).
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(value: f32) -> Self {
        Kilometers(value)
    }

    /// Convert to meters
    #[inline]
    #[must_use]
    pub fn to_meters(self) -> Meters {
        Meters::new(self.0 * 1000.0)
    }

    /// Get the raw f32 value
    #[inline]
    #[must_use]
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
pub struct Kilograms(f32);

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

impl Deref for Kilograms {
    type Target = f32;
    #[inline]
    fn deref(&self) -> &f32 {
        &self.0
    }
}

impl DerefMut for Kilograms {
    #[inline]
    fn deref_mut(&mut self) -> &mut f32 {
        &mut self.0
    }
}

impl Kilograms {
    /// Create a new mass in kilograms. Asserts value >= 0 (non-negative mass).
    #[inline]
    #[must_use]
    pub const fn new(value: f32) -> Self {
        assert!(value >= 0.0, "Kilograms::new: negative mass is invalid");
        Kilograms(value)
    }

    /// Create without validation.
    /// # Safety
    /// Caller must ensure value >= 0 (non-negative mass).
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(value: f32) -> Self {
        Kilograms(value)
    }

    /// Get the raw f32 value
    #[inline]
    #[must_use]
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
        // Pure addition - if inputs are valid, output is valid
        Kilograms(self.0 + rhs.0)
    }
}

impl Sub for Kilograms {
    type Output = Kilograms;
    fn sub(self, rhs: Kilograms) -> Kilograms {
        let result = self.0 - rhs.0;
        assert!(result >= 0.0, "Negative mass: {result:.6} kg");
        Kilograms(result)
    }
}

impl SubAssign<f32> for Kilograms {
    fn sub_assign(&mut self, rhs: f32) {
        // Consumption - clamp to 0 since mass can't be negative
        self.0 = (self.0 - rhs).max(0.0);
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
        // Pure multiplication - if input is valid and scalar >= 0, output is valid
        Kilograms(self.0 * rhs)
    }
}

impl Mul<Kilograms> for f32 {
    type Output = Kilograms;
    fn mul(self, rhs: Kilograms) -> Kilograms {
        // Pure multiplication - if input is valid and scalar >= 0, output is valid
        Kilograms(self * rhs.0)
    }
}

impl Div<f32> for Kilograms {
    type Output = Kilograms;
    fn div(self, rhs: f32) -> Kilograms {
        // Pure division - if input is valid and scalar > 0, output is valid
        Kilograms(self.0 / rhs)
    }
}

/// Density in kg/m³
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct KgPerCubicMeter(f32);

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

impl Deref for KgPerCubicMeter {
    type Target = f32;
    #[inline]
    fn deref(&self) -> &f32 {
        &self.0
    }
}

impl DerefMut for KgPerCubicMeter {
    #[inline]
    fn deref_mut(&mut self) -> &mut f32 {
        &mut self.0
    }
}

impl KgPerCubicMeter {
    /// Density of water at 4°C
    pub const WATER: KgPerCubicMeter = KgPerCubicMeter(1000.0);

    /// Density of air at sea level, 15°C
    pub const AIR_SEA_LEVEL: KgPerCubicMeter = KgPerCubicMeter(1.225);

    /// Create a new density. Asserts value >= 0 (non-negative density).
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn new(value: f32) -> Self {
        assert!(
            value >= 0.0,
            "KgPerCubicMeter::new: negative density is invalid"
        );
        KgPerCubicMeter(value)
    }

    /// Create without validation.
    /// # Safety
    /// Caller must ensure value >= 0 (non-negative density).
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(value: f32) -> Self {
        KgPerCubicMeter(value)
    }

    /// Get the raw f32 value
    #[inline]
    #[must_use]
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

/// Fuel load per unit area in kg/m²
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct KilogramsPerSquareMeter(f32);

impl Eq for KilogramsPerSquareMeter {}

impl PartialOrd for KilogramsPerSquareMeter {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for KilogramsPerSquareMeter {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl Deref for KilogramsPerSquareMeter {
    type Target = f32;
    #[inline]
    fn deref(&self) -> &f32 {
        &self.0
    }
}

impl DerefMut for KilogramsPerSquareMeter {
    #[inline]
    fn deref_mut(&mut self) -> &mut f32 {
        &mut self.0
    }
}

impl KilogramsPerSquareMeter {
    /// Typical grass fuel load
    pub const GRASS: KilogramsPerSquareMeter = KilogramsPerSquareMeter(0.5);

    /// Typical forest fuel load
    pub const FOREST: KilogramsPerSquareMeter = KilogramsPerSquareMeter(5.0);

    /// Extreme fuel load conditions
    pub const EXTREME: KilogramsPerSquareMeter = KilogramsPerSquareMeter(15.0);

    /// Create a new fuel load. Asserts value >= 0 (non-negative fuel load).
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn new(value: f32) -> Self {
        assert!(
            value >= 0.0,
            "KilogramsPerSquareMeter::new: negative fuel load is invalid"
        );
        KilogramsPerSquareMeter(value)
    }

    /// Create without validation.
    /// # Safety
    /// Caller must ensure value >= 0 (non-negative fuel load).
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(value: f32) -> Self {
        KilogramsPerSquareMeter(value)
    }

    /// Get the raw f32 value
    #[inline]
    #[must_use]
    pub fn value(self) -> f32 {
        self.0
    }
}

impl From<f32> for KilogramsPerSquareMeter {
    fn from(v: f32) -> Self {
        KilogramsPerSquareMeter(v)
    }
}

impl From<KilogramsPerSquareMeter> for f32 {
    fn from(k: KilogramsPerSquareMeter) -> f32 {
        k.0
    }
}

impl Add for KilogramsPerSquareMeter {
    type Output = KilogramsPerSquareMeter;
    fn add(self, rhs: KilogramsPerSquareMeter) -> KilogramsPerSquareMeter {
        // Clamped at 0 to prevent negative fuel load
        KilogramsPerSquareMeter((self.0 + rhs.0).max(0.0))
    }
}

impl Sub for KilogramsPerSquareMeter {
    type Output = KilogramsPerSquareMeter;
    fn sub(self, rhs: KilogramsPerSquareMeter) -> KilogramsPerSquareMeter {
        // Clamped at 0 to prevent negative fuel load
        KilogramsPerSquareMeter((self.0 - rhs.0).max(0.0))
    }
}

impl Mul<Fraction> for KilogramsPerSquareMeter {
    type Output = KilogramsPerSquareMeter;
    fn mul(self, rhs: Fraction) -> KilogramsPerSquareMeter {
        KilogramsPerSquareMeter(self.0 * rhs.0)
    }
}

impl Mul<KilogramsPerSquareMeter> for Fraction {
    type Output = KilogramsPerSquareMeter;
    fn mul(self, rhs: KilogramsPerSquareMeter) -> KilogramsPerSquareMeter {
        KilogramsPerSquareMeter(self.0 * rhs.0)
    }
}

impl Div<Fraction> for KilogramsPerSquareMeter {
    type Output = KilogramsPerSquareMeter;
    fn div(self, rhs: Fraction) -> KilogramsPerSquareMeter {
        KilogramsPerSquareMeter(self.0 / rhs.0)
    }
}

impl fmt::Display for KilogramsPerSquareMeter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} kg/m²", self.0)
    }
}

// ============================================================================
// TIME TYPES
// ============================================================================

/// Time duration in seconds
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Seconds(f32);

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

impl Deref for Seconds {
    type Target = f32;
    #[inline]
    fn deref(&self) -> &f32 {
        &self.0
    }
}

impl DerefMut for Seconds {
    #[inline]
    fn deref_mut(&mut self) -> &mut f32 {
        &mut self.0
    }
}

impl Seconds {
    /// Create a new duration in seconds
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn new(value: f32) -> Self {
        assert!(value >= 0.0, "Seconds::new: negative duration is invalid");
        Seconds(value)
    }

    /// Create without validation.
    /// # Safety
    /// Caller must ensure value >= 0 (non-negative duration).
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(value: f32) -> Self {
        Seconds(value)
    }

    /// Get the raw f32 value
    #[inline]
    #[must_use]
    pub fn value(self) -> f32 {
        self.0
    }

    /// Convert to hours
    #[inline]
    #[must_use]
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
pub struct Hours(f32);

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

impl Deref for Hours {
    type Target = f32;
    #[inline]
    fn deref(&self) -> &f32 {
        &self.0
    }
}

impl DerefMut for Hours {
    #[inline]
    fn deref_mut(&mut self) -> &mut f32 {
        &mut self.0
    }
}

impl Hours {
    /// Create a new Hours value.
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn new(value: f32) -> Self {
        assert!(value >= 0.0, "Hours::new: negative duration is invalid");
        Hours(value)
    }

    /// Create without validation.
    /// # Safety
    /// No validation required for time.
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(value: f32) -> Self {
        Hours(value)
    }

    /// Convert to seconds
    #[inline]
    #[must_use]
    pub fn to_seconds(self) -> Seconds {
        Seconds::new(self.0 * 3600.0)
    }

    /// Get the raw f32 value
    #[inline]
    #[must_use]
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
        Hours::new(v)
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
pub struct MetersPerSecond(f32);

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

impl Deref for MetersPerSecond {
    type Target = f32;
    #[inline]
    fn deref(&self) -> &f32 {
        &self.0
    }
}

impl DerefMut for MetersPerSecond {
    #[inline]
    fn deref_mut(&mut self) -> &mut f32 {
        &mut self.0
    }
}

impl MetersPerSecond {
    /// Create a new velocity
    #[inline]
    #[must_use]
    pub const fn new(value: f32) -> Self {
        MetersPerSecond(value)
    }

    /// Get the raw f32 value
    #[inline]
    #[must_use]
    pub fn value(self) -> f32 {
        self.0
    }

    /// Convert to km/h
    #[inline]
    #[must_use]
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
pub struct KilometersPerHour(f32);

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

impl Deref for KilometersPerHour {
    type Target = f32;
    #[inline]
    fn deref(&self) -> &f32 {
        &self.0
    }
}

impl DerefMut for KilometersPerHour {
    #[inline]
    fn deref_mut(&mut self) -> &mut f32 {
        &mut self.0
    }
}

impl KilometersPerHour {
    /// Create a new `KilometersPerHour` value.
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn new(value: f32) -> Self {
        KilometersPerHour(value)
    }

    /// Convert to m/s
    #[inline]
    #[must_use]
    pub fn to_mps(self) -> MetersPerSecond {
        MetersPerSecond::new(self.0 / 3.6)
    }

    /// Get the raw f32 value
    #[inline]
    #[must_use]
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
        KilometersPerHour::new(v)
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

/// Fire spread rate in meters per minute
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct MetersPerMinute(f32);

impl Eq for MetersPerMinute {}

impl PartialOrd for MetersPerMinute {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for MetersPerMinute {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl Deref for MetersPerMinute {
    type Target = f32;
    #[inline]
    fn deref(&self) -> &f32 {
        &self.0
    }
}

impl DerefMut for MetersPerMinute {
    #[inline]
    fn deref_mut(&mut self) -> &mut f32 {
        &mut self.0
    }
}

impl MetersPerMinute {
    /// Conversion factor: meters per minute to meters per second (divide by 60)
    const TO_MPS_FACTOR: f32 = 1.0 / 60.0;

    /// Conversion factor: meters per second to meters per minute (multiply by 60)
    const FROM_MPS_FACTOR: f32 = 60.0;

    /// Create a new fire spread rate. Asserts value >= 0.
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn new(value: f32) -> Self {
        assert!(
            value >= 0.0,
            "MetersPerMinute::new: negative spread rate is invalid"
        );
        MetersPerMinute(value)
    }

    /// Create without validation.
    /// # Safety
    /// Caller must ensure value >= 0.
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(value: f32) -> Self {
        MetersPerMinute(value)
    }

    /// Get the raw f32 value
    #[inline]
    #[must_use]
    pub fn value(self) -> f32 {
        self.0
    }

    /// Convert to meters per second
    #[inline]
    #[must_use]
    pub fn to_mps(self) -> MetersPerSecond {
        MetersPerSecond(self.0 * Self::TO_MPS_FACTOR)
    }

    /// Create from meters per second
    #[inline]
    #[must_use]
    pub fn from_mps(mps: MetersPerSecond) -> Self {
        MetersPerMinute(mps.0 * Self::FROM_MPS_FACTOR)
    }
}

impl From<f32> for MetersPerMinute {
    fn from(v: f32) -> Self {
        MetersPerMinute(v)
    }
}

impl From<MetersPerMinute> for f32 {
    fn from(m: MetersPerMinute) -> f32 {
        m.0
    }
}

impl From<MetersPerMinute> for MetersPerSecond {
    fn from(m: MetersPerMinute) -> MetersPerSecond {
        m.to_mps()
    }
}

impl From<MetersPerSecond> for MetersPerMinute {
    fn from(mps: MetersPerSecond) -> MetersPerMinute {
        MetersPerMinute::from_mps(mps)
    }
}

impl Add for MetersPerMinute {
    type Output = MetersPerMinute;
    fn add(self, rhs: MetersPerMinute) -> MetersPerMinute {
        MetersPerMinute(self.0 + rhs.0)
    }
}

impl Sub for MetersPerMinute {
    type Output = MetersPerMinute;
    fn sub(self, rhs: MetersPerMinute) -> MetersPerMinute {
        MetersPerMinute(self.0 - rhs.0)
    }
}

impl Mul<f32> for MetersPerMinute {
    type Output = MetersPerMinute;
    fn mul(self, rhs: f32) -> MetersPerMinute {
        MetersPerMinute(self.0 * rhs)
    }
}

impl Div<f32> for MetersPerMinute {
    type Output = MetersPerMinute;
    fn div(self, rhs: f32) -> MetersPerMinute {
        MetersPerMinute(self.0 / rhs)
    }
}

impl fmt::Display for MetersPerMinute {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} m/min", self.0)
    }
}

// ============================================================================
// ENERGY/POWER TYPES
// ============================================================================

/// Energy in kilojoules
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Kilojoules(f32);

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

impl Deref for Kilojoules {
    type Target = f32;
    #[inline]
    fn deref(&self) -> &f32 {
        &self.0
    }
}

impl DerefMut for Kilojoules {
    #[inline]
    fn deref_mut(&mut self) -> &mut f32 {
        &mut self.0
    }
}

impl Kilojoules {
    /// Create a new energy value
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn new(value: f32) -> Self {
        assert!(value >= 0.0, "Kilojoules::new: negative energy is invalid");
        Kilojoules(value)
    }

    /// Create without validation.
    /// # Safety
    /// No validation required for energy.
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(value: f32) -> Self {
        Kilojoules(value)
    }

    /// Get the raw f32 value
    #[inline]
    #[must_use]
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

/// Power in gigawatts (GW)
/// Used for measuring total fire power output from large fires
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Gigawatts(f32);

impl Eq for Gigawatts {}

impl PartialOrd for Gigawatts {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Gigawatts {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl Deref for Gigawatts {
    type Target = f32;
    #[inline]
    fn deref(&self) -> &f32 {
        &self.0
    }
}

impl DerefMut for Gigawatts {
    #[inline]
    fn deref_mut(&mut self) -> &mut f32 {
        &mut self.0
    }
}

impl Gigawatts {
    /// Create a new power value in gigawatts
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn new(value: f32) -> Self {
        assert!(value >= 0.0, "Gigawatts::new: negative power is invalid");
        Gigawatts(value)
    }

    /// Create without validation.
    /// # Safety
    /// Caller must ensure value >= 0 (non-negative power).
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(value: f32) -> Self {
        Gigawatts(value)
    }

    /// Get the raw f32 value
    #[inline]
    #[must_use]
    pub fn value(self) -> f32 {
        self.0
    }
}

impl From<f32> for Gigawatts {
    fn from(v: f32) -> Self {
        Gigawatts(v)
    }
}

impl From<Gigawatts> for f32 {
    fn from(gw: Gigawatts) -> f32 {
        gw.0
    }
}

impl Add for Gigawatts {
    type Output = Gigawatts;
    fn add(self, rhs: Gigawatts) -> Gigawatts {
        Gigawatts(self.0 + rhs.0)
    }
}

impl Sub for Gigawatts {
    type Output = Gigawatts;
    fn sub(self, rhs: Gigawatts) -> Gigawatts {
        Gigawatts(self.0 - rhs.0)
    }
}

impl Mul<f32> for Gigawatts {
    type Output = Gigawatts;
    fn mul(self, rhs: f32) -> Gigawatts {
        Gigawatts(self.0 * rhs)
    }
}

impl Div<f32> for Gigawatts {
    type Output = Gigawatts;
    fn div(self, rhs: f32) -> Gigawatts {
        Gigawatts(self.0 / rhs)
    }
}

impl PartialEq<f32> for Gigawatts {
    fn eq(&self, other: &f32) -> bool {
        self.0 == *other
    }
}

impl PartialOrd<f32> for Gigawatts {
    fn partial_cmp(&self, other: &f32) -> Option<std::cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

impl fmt::Display for Gigawatts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.2} GW", self.0)
    }
}

/// Heat content in kJ/kg
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct KjPerKg(f32);

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

impl Deref for KjPerKg {
    type Target = f32;
    #[inline]
    fn deref(&self) -> &f32 {
        &self.0
    }
}

impl DerefMut for KjPerKg {
    #[inline]
    fn deref_mut(&mut self) -> &mut f32 {
        &mut self.0
    }
}

impl KjPerKg {
    /// Latent heat of vaporization for water (2260 kJ/kg)
    pub const WATER_LATENT_HEAT: KjPerKg = KjPerKg(2260.0);

    /// Create a new heat content value
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn new(value: f32) -> Self {
        assert!(
            value >= 0.0,
            "KjPerKg::new: negative heat content is invalid"
        );
        KjPerKg(value)
    }

    /// Create without validation.
    /// # Safety
    /// No validation required for specific energy.
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(value: f32) -> Self {
        KjPerKg(value)
    }

    /// Get the raw f32 value
    #[inline]
    #[must_use]
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

// ============================================================================
// SPECIFIC HEAT TYPE
// ============================================================================

/// Specific heat capacity in kJ/(kg·K)
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct KjPerKgK(f32);

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

impl Deref for KjPerKgK {
    type Target = f32;
    #[inline]
    fn deref(&self) -> &f32 {
        &self.0
    }
}

impl DerefMut for KjPerKgK {
    #[inline]
    fn deref_mut(&mut self) -> &mut f32 {
        &mut self.0
    }
}

impl KjPerKgK {
    /// Specific heat of water (4.18 kJ/(kg·K))
    pub const WATER: KjPerKgK = KjPerKgK(4.18);

    /// Specific heat of dry wood (approximately 1.5 kJ/(kg·K))
    pub const DRY_WOOD: KjPerKgK = KjPerKgK(1.5);

    /// Specific heat of air at constant pressure (1.005 kJ/(kg·K))
    pub const AIR: KjPerKgK = KjPerKgK(1.005);

    /// Create a new specific heat value. Asserts value >= 0 (non-negative specific heat).
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn new(value: f32) -> Self {
        assert!(
            value >= 0.0,
            "KjPerKgK::new: negative specific heat is invalid"
        );
        KjPerKgK(value)
    }

    /// Create without validation.
    /// # Safety
    /// Caller must ensure value >= 0 (non-negative specific heat).
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(value: f32) -> Self {
        KjPerKgK(value)
    }

    /// Get the raw f32 value
    #[inline]
    #[must_use]
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
// THERMAL PROPERTIES
// ============================================================================

/// Thermal conductivity in W/(m·K)
/// Measures a material's ability to conduct heat
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct ThermalConductivity(f32);

impl Eq for ThermalConductivity {}

impl PartialOrd for ThermalConductivity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ThermalConductivity {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl Deref for ThermalConductivity {
    type Target = f32;
    #[inline]
    fn deref(&self) -> &f32 {
        &self.0
    }
}

impl DerefMut for ThermalConductivity {
    #[inline]
    fn deref_mut(&mut self) -> &mut f32 {
        &mut self.0
    }
}

impl ThermalConductivity {
    /// Thermal conductivity of wood (typical range 0.1-0.2 W/(m·K))
    pub const WOOD_TYPICAL: ThermalConductivity = ThermalConductivity(0.15);

    /// Thermal conductivity of air at 20°C (0.025 W/(m·K))
    pub const AIR: ThermalConductivity = ThermalConductivity(0.025);

    /// Thermal conductivity of soil (typical range 0.5-2.0 W/(m·K))
    pub const SOIL_TYPICAL: ThermalConductivity = ThermalConductivity(1.0);

    /// Create a new thermal conductivity value. Asserts value >= 0.
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn new(value: f32) -> Self {
        assert!(
            value >= 0.0,
            "ThermalConductivity::new: negative thermal conductivity is invalid"
        );
        ThermalConductivity(value)
    }

    /// Create without validation.
    /// # Safety
    /// Caller must ensure value >= 0.
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(value: f32) -> Self {
        ThermalConductivity(value)
    }

    /// Get the raw f32 value
    #[inline]
    #[must_use]
    pub fn value(self) -> f32 {
        self.0
    }
}

impl From<f32> for ThermalConductivity {
    fn from(v: f32) -> Self {
        ThermalConductivity(v)
    }
}

impl From<ThermalConductivity> for f32 {
    fn from(k: ThermalConductivity) -> f32 {
        k.0
    }
}

impl Add for ThermalConductivity {
    type Output = ThermalConductivity;
    fn add(self, rhs: ThermalConductivity) -> ThermalConductivity {
        ThermalConductivity(self.0 + rhs.0)
    }
}

impl Sub for ThermalConductivity {
    type Output = ThermalConductivity;
    fn sub(self, rhs: ThermalConductivity) -> ThermalConductivity {
        ThermalConductivity(self.0 - rhs.0)
    }
}

impl Mul<f32> for ThermalConductivity {
    type Output = ThermalConductivity;
    fn mul(self, rhs: f32) -> ThermalConductivity {
        ThermalConductivity(self.0 * rhs)
    }
}

impl Div<f32> for ThermalConductivity {
    type Output = ThermalConductivity;
    fn div(self, rhs: f32) -> ThermalConductivity {
        ThermalConductivity(self.0 / rhs)
    }
}

impl fmt::Display for ThermalConductivity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} W/(m·K)", self.0)
    }
}

/// Thermal diffusivity in m²/s
/// Measures the rate of heat transfer through a material
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct ThermalDiffusivity(f32);

impl Eq for ThermalDiffusivity {}

impl PartialOrd for ThermalDiffusivity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ThermalDiffusivity {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl Deref for ThermalDiffusivity {
    type Target = f32;
    #[inline]
    fn deref(&self) -> &f32 {
        &self.0
    }
}

impl DerefMut for ThermalDiffusivity {
    #[inline]
    fn deref_mut(&mut self) -> &mut f32 {
        &mut self.0
    }
}

impl ThermalDiffusivity {
    /// Thermal diffusivity of wood (typical ~1e-7 m²/s)
    pub const WOOD: ThermalDiffusivity = ThermalDiffusivity(1e-7);

    /// Thermal diffusivity of air at 20°C (~2e-5 m²/s)
    pub const AIR: ThermalDiffusivity = ThermalDiffusivity(2e-5);

    /// Thermal diffusivity of soil (typical ~5e-7 m²/s)
    pub const SOIL: ThermalDiffusivity = ThermalDiffusivity(5e-7);

    /// Create a new thermal diffusivity value. Asserts value >= 0.
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn new(value: f32) -> Self {
        assert!(
            value >= 0.0,
            "ThermalDiffusivity::new: negative thermal diffusivity is invalid"
        );
        ThermalDiffusivity(value)
    }

    /// Create without validation.
    /// # Safety
    /// Caller must ensure value >= 0.
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(value: f32) -> Self {
        ThermalDiffusivity(value)
    }

    /// Get the raw f32 value
    #[inline]
    #[must_use]
    pub fn value(self) -> f32 {
        self.0
    }
}

impl From<f32> for ThermalDiffusivity {
    fn from(v: f32) -> Self {
        ThermalDiffusivity(v)
    }
}

impl From<ThermalDiffusivity> for f32 {
    fn from(a: ThermalDiffusivity) -> f32 {
        a.0
    }
}

impl Add for ThermalDiffusivity {
    type Output = ThermalDiffusivity;
    fn add(self, rhs: ThermalDiffusivity) -> ThermalDiffusivity {
        ThermalDiffusivity(self.0 + rhs.0)
    }
}

impl Sub for ThermalDiffusivity {
    type Output = ThermalDiffusivity;
    fn sub(self, rhs: ThermalDiffusivity) -> ThermalDiffusivity {
        ThermalDiffusivity(self.0 - rhs.0)
    }
}

impl Mul<f32> for ThermalDiffusivity {
    type Output = ThermalDiffusivity;
    fn mul(self, rhs: f32) -> ThermalDiffusivity {
        ThermalDiffusivity(self.0 * rhs)
    }
}

impl Div<f32> for ThermalDiffusivity {
    type Output = ThermalDiffusivity;
    fn div(self, rhs: f32) -> ThermalDiffusivity {
        ThermalDiffusivity(self.0 / rhs)
    }
}

impl fmt::Display for ThermalDiffusivity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} m²/s", self.0)
    }
}

/// Heat transfer coefficient in W/(m²·K)
/// Measures convective heat transfer at a surface
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct HeatTransferCoefficient(f32);

impl Eq for HeatTransferCoefficient {}

impl PartialOrd for HeatTransferCoefficient {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for HeatTransferCoefficient {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl Deref for HeatTransferCoefficient {
    type Target = f32;
    #[inline]
    fn deref(&self) -> &f32 {
        &self.0
    }
}

impl DerefMut for HeatTransferCoefficient {
    #[inline]
    fn deref_mut(&mut self) -> &mut f32 {
        &mut self.0
    }
}

impl HeatTransferCoefficient {
    /// Natural convection (typical range 5-25 W/(m²·K))
    pub const NATURAL_CONVECTION: HeatTransferCoefficient = HeatTransferCoefficient(15.0);

    /// Forced convection (typical range 25-250 W/(m²·K))
    pub const FORCED_CONVECTION: HeatTransferCoefficient = HeatTransferCoefficient(100.0);

    /// Create a new heat transfer coefficient. Asserts value >= 0.
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn new(value: f32) -> Self {
        assert!(
            value >= 0.0,
            "HeatTransferCoefficient::new: negative heat transfer coefficient is invalid"
        );
        HeatTransferCoefficient(value)
    }

    /// Create without validation.
    /// # Safety
    /// Caller must ensure value >= 0.
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(value: f32) -> Self {
        HeatTransferCoefficient(value)
    }

    /// Get the raw f32 value
    #[inline]
    #[must_use]
    pub fn value(self) -> f32 {
        self.0
    }
}

impl From<f32> for HeatTransferCoefficient {
    fn from(v: f32) -> Self {
        HeatTransferCoefficient(v)
    }
}

impl From<HeatTransferCoefficient> for f32 {
    fn from(h: HeatTransferCoefficient) -> f32 {
        h.0
    }
}

impl Add for HeatTransferCoefficient {
    type Output = HeatTransferCoefficient;
    fn add(self, rhs: HeatTransferCoefficient) -> HeatTransferCoefficient {
        HeatTransferCoefficient(self.0 + rhs.0)
    }
}

impl Sub for HeatTransferCoefficient {
    type Output = HeatTransferCoefficient;
    fn sub(self, rhs: HeatTransferCoefficient) -> HeatTransferCoefficient {
        HeatTransferCoefficient(self.0 - rhs.0)
    }
}

impl Mul<f32> for HeatTransferCoefficient {
    type Output = HeatTransferCoefficient;
    fn mul(self, rhs: f32) -> HeatTransferCoefficient {
        HeatTransferCoefficient(self.0 * rhs)
    }
}

impl Div<f32> for HeatTransferCoefficient {
    type Output = HeatTransferCoefficient;
    fn div(self, rhs: f32) -> HeatTransferCoefficient {
        HeatTransferCoefficient(self.0 / rhs)
    }
}

impl fmt::Display for HeatTransferCoefficient {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} W/(m²·K)", self.0)
    }
}

/// Heat flux density and solar irradiance in W/m²
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct WattsPerSquareMeter(f32);

impl Eq for WattsPerSquareMeter {}

impl PartialOrd for WattsPerSquareMeter {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for WattsPerSquareMeter {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl Deref for WattsPerSquareMeter {
    type Target = f32;
    #[inline]
    fn deref(&self) -> &f32 {
        &self.0
    }
}

impl DerefMut for WattsPerSquareMeter {
    #[inline]
    fn deref_mut(&mut self) -> &mut f32 {
        &mut self.0
    }
}

impl WattsPerSquareMeter {
    /// Maximum solar irradiance at sea level (noon, clear sky)
    pub const SOLAR_MAX: WattsPerSquareMeter = WattsPerSquareMeter(1000.0);

    /// Typical minimum solar irradiance for daylight
    pub const SOLAR_TYPICAL_MIN: WattsPerSquareMeter = WattsPerSquareMeter(200.0);

    /// Typical maximum solar irradiance for normal conditions
    pub const SOLAR_TYPICAL_MAX: WattsPerSquareMeter = WattsPerSquareMeter(600.0);

    /// Create a new heat flux. Asserts value >= 0 (non-negative heat flux).
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn new(value: f32) -> Self {
        assert!(
            value >= 0.0,
            "WattsPerSquareMeter::new: negative heat flux is invalid"
        );
        WattsPerSquareMeter(value)
    }

    /// Create without validation.
    /// # Safety
    /// Caller must ensure value >= 0 (non-negative heat flux).
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(value: f32) -> Self {
        WattsPerSquareMeter(value)
    }

    /// Get the raw f32 value
    #[inline]
    #[must_use]
    pub fn value(self) -> f32 {
        self.0
    }
}

impl From<f32> for WattsPerSquareMeter {
    fn from(v: f32) -> Self {
        WattsPerSquareMeter(v)
    }
}

impl From<WattsPerSquareMeter> for f32 {
    fn from(w: WattsPerSquareMeter) -> f32 {
        w.0
    }
}

impl Add for WattsPerSquareMeter {
    type Output = WattsPerSquareMeter;
    fn add(self, rhs: WattsPerSquareMeter) -> WattsPerSquareMeter {
        WattsPerSquareMeter(self.0 + rhs.0)
    }
}

impl Sub for WattsPerSquareMeter {
    type Output = WattsPerSquareMeter;
    fn sub(self, rhs: WattsPerSquareMeter) -> WattsPerSquareMeter {
        WattsPerSquareMeter(self.0 - rhs.0)
    }
}

impl Mul<f32> for WattsPerSquareMeter {
    type Output = WattsPerSquareMeter;
    fn mul(self, rhs: f32) -> WattsPerSquareMeter {
        WattsPerSquareMeter(self.0 * rhs)
    }
}

impl Mul<WattsPerSquareMeter> for f32 {
    type Output = WattsPerSquareMeter;
    fn mul(self, rhs: WattsPerSquareMeter) -> WattsPerSquareMeter {
        WattsPerSquareMeter(self * rhs.0)
    }
}

impl Div<f32> for WattsPerSquareMeter {
    type Output = WattsPerSquareMeter;
    fn div(self, rhs: f32) -> WattsPerSquareMeter {
        WattsPerSquareMeter(self.0 / rhs)
    }
}

// Cross-type operation: W/m² × seconds = J/m²
impl Mul<Seconds> for WattsPerSquareMeter {
    type Output = JoulesPerSquareMeter;
    fn mul(self, rhs: Seconds) -> JoulesPerSquareMeter {
        JoulesPerSquareMeter(self.0 * rhs.0)
    }
}

// Cross-type operation: seconds × W/m² = J/m²
impl Mul<WattsPerSquareMeter> for Seconds {
    type Output = JoulesPerSquareMeter;
    fn mul(self, rhs: WattsPerSquareMeter) -> JoulesPerSquareMeter {
        JoulesPerSquareMeter(self.0 * rhs.0)
    }
}

impl fmt::Display for WattsPerSquareMeter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} W/m²", self.0)
    }
}

/// Energy per unit area in J/m²
/// Created by multiplying [`WattsPerSquareMeter`] × [`Seconds`]
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct JoulesPerSquareMeter(f32);

impl Eq for JoulesPerSquareMeter {}

impl PartialOrd for JoulesPerSquareMeter {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for JoulesPerSquareMeter {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl Deref for JoulesPerSquareMeter {
    type Target = f32;
    #[inline]
    fn deref(&self) -> &f32 {
        &self.0
    }
}

impl DerefMut for JoulesPerSquareMeter {
    #[inline]
    fn deref_mut(&mut self) -> &mut f32 {
        &mut self.0
    }
}

impl JoulesPerSquareMeter {
    /// Create a new energy per unit area. Asserts value >= 0.
    #[inline]
    #[must_use]
    #[track_caller]
    pub const fn new(value: f32) -> Self {
        assert!(
            value >= 0.0,
            "JoulesPerSquareMeter::new: negative energy density is invalid"
        );
        JoulesPerSquareMeter(value)
    }

    /// Create without validation.
    /// # Safety
    /// Caller must ensure value >= 0.
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(value: f32) -> Self {
        JoulesPerSquareMeter(value)
    }

    /// Get the raw f32 value
    #[inline]
    #[must_use]
    pub fn value(self) -> f32 {
        self.0
    }
}

impl From<f32> for JoulesPerSquareMeter {
    fn from(v: f32) -> Self {
        JoulesPerSquareMeter(v)
    }
}

impl From<JoulesPerSquareMeter> for f32 {
    fn from(j: JoulesPerSquareMeter) -> f32 {
        j.0
    }
}

impl Add for JoulesPerSquareMeter {
    type Output = JoulesPerSquareMeter;
    fn add(self, rhs: JoulesPerSquareMeter) -> JoulesPerSquareMeter {
        JoulesPerSquareMeter(self.0 + rhs.0)
    }
}

impl Sub for JoulesPerSquareMeter {
    type Output = JoulesPerSquareMeter;
    fn sub(self, rhs: JoulesPerSquareMeter) -> JoulesPerSquareMeter {
        JoulesPerSquareMeter(self.0 - rhs.0)
    }
}

impl Mul<f32> for JoulesPerSquareMeter {
    type Output = JoulesPerSquareMeter;
    fn mul(self, rhs: f32) -> JoulesPerSquareMeter {
        JoulesPerSquareMeter(self.0 * rhs)
    }
}

impl Div<f32> for JoulesPerSquareMeter {
    type Output = JoulesPerSquareMeter;
    fn div(self, rhs: f32) -> JoulesPerSquareMeter {
        JoulesPerSquareMeter(self.0 / rhs)
    }
}

// Cross-type operation: J/m² ÷ seconds = W/m²
impl Div<Seconds> for JoulesPerSquareMeter {
    type Output = WattsPerSquareMeter;
    fn div(self, rhs: Seconds) -> WattsPerSquareMeter {
        WattsPerSquareMeter(self.0 / rhs.0)
    }
}

impl fmt::Display for JoulesPerSquareMeter {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} J/m²", self.0)
    }
}

// ============================================================================
// FRACTION/RATIO TYPES
// ============================================================================

/// A fraction in the range [0, 1]
/// Represents moisture content, efficiency ratios, damping coefficients, etc.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct Fraction(f32);

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

impl Deref for Fraction {
    type Target = f32;
    #[inline]
    fn deref(&self) -> &f32 {
        &self.0
    }
}

impl DerefMut for Fraction {
    #[inline]
    fn deref_mut(&mut self) -> &mut f32 {
        &mut self.0
    }
}

impl Fraction {
    /// Zero fraction
    pub const ZERO: Fraction = Fraction(0.0);

    /// Full/complete (1.0)
    pub const ONE: Fraction = Fraction(1.0);

    /// Create a new fraction. Asserts value is within [0, 1].
    #[inline]
    #[must_use]
    pub const fn new(value: f32) -> Self {
        assert!(
            value >= 0.0 && value <= 1.0,
            "Fraction::new: value not in [0, 1]"
        );
        Fraction(value)
    }

    /// Create a fraction without clamping (for performance when value is known valid)
    /// # Safety
    /// Caller must ensure value is in [0, 1].
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(value: f32) -> Self {
        Fraction(value)
    }

    /// Get the raw f32 value
    #[inline]
    #[must_use]
    pub fn value(self) -> f32 {
        self.0
    }

    /// Convert to percentage (0-100)
    #[inline]
    #[must_use]
    pub fn to_percent(self) -> Percent {
        Percent::new(self.0 * 100.0)
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
pub struct Percent(f32);

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

impl Deref for Percent {
    type Target = f32;
    #[inline]
    fn deref(&self) -> &f32 {
        &self.0
    }
}

impl DerefMut for Percent {
    #[inline]
    fn deref_mut(&mut self) -> &mut f32 {
        &mut self.0
    }
}

impl Percent {
    /// Create a new percentage
    #[inline]
    #[must_use]
    pub const fn new(value: f32) -> Self {
        Percent(value)
    }

    /// Get the raw f32 value
    #[inline]
    #[must_use]
    pub fn value(self) -> f32 {
        self.0
    }

    /// Convert to fraction (0-1)
    #[inline]
    #[must_use]
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
pub struct Degrees(f32);

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

impl Deref for Degrees {
    type Target = f32;
    #[inline]
    fn deref(&self) -> &f32 {
        &self.0
    }
}

impl DerefMut for Degrees {
    #[inline]
    fn deref_mut(&mut self) -> &mut f32 {
        &mut self.0
    }
}

impl Degrees {
    /// Create a new angle in degrees
    #[inline]
    #[must_use]
    pub const fn new(value: f32) -> Self {
        Degrees(value)
    }

    /// Get the raw f32 value
    #[inline]
    #[must_use]
    pub fn value(self) -> f32 {
        self.0
    }

    /// Convert to radians
    #[inline]
    #[must_use]
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
pub struct Radians(f32);

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

impl Deref for Radians {
    type Target = f32;
    #[inline]
    fn deref(&self) -> &f32 {
        &self.0
    }
}

impl DerefMut for Radians {
    #[inline]
    fn deref_mut(&mut self) -> &mut f32 {
        &mut self.0
    }
}

impl Radians {
    /// Create a new angle in radians
    #[inline]
    #[must_use]
    pub const fn new(value: f32) -> Self {
        Radians(value)
    }

    /// Get the raw f32 value
    #[inline]
    #[must_use]
    pub fn value(self) -> f32 {
        self.0
    }

    /// Convert to degrees
    #[inline]
    #[must_use]
    pub fn to_degrees(self) -> Degrees {
        Degrees(self.0.to_degrees())
    }

    /// Compute sine
    #[inline]
    #[must_use]
    pub fn sin(self) -> f32 {
        self.0.sin()
    }

    /// Compute cosine
    #[inline]
    #[must_use]
    pub fn cos(self) -> f32 {
        self.0.cos()
    }

    /// Compute tangent
    #[inline]
    #[must_use]
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
// RATE TYPES (TEMPORAL)
// ============================================================================

/// Rate per second (1/s or s⁻¹)
/// Used for: cooling rates, vorticity, fire whirl thresholds
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct RatePerSecond(f32);

impl Eq for RatePerSecond {}

impl PartialOrd for RatePerSecond {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RatePerSecond {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl Deref for RatePerSecond {
    type Target = f32;
    #[inline]
    fn deref(&self) -> &f32 {
        &self.0
    }
}

impl DerefMut for RatePerSecond {
    #[inline]
    fn deref_mut(&mut self) -> &mut f32 {
        &mut self.0
    }
}

impl RatePerSecond {
    /// Create a new rate per second (can be any value, positive or negative)
    #[inline]
    #[must_use]
    pub const fn new(value: f32) -> Self {
        RatePerSecond(value)
    }

    /// Get the raw f32 value
    #[inline]
    #[must_use]
    pub fn value(self) -> f32 {
        self.0
    }

    /// Convert to rate per day
    #[inline]
    #[must_use]
    pub fn to_rate_per_day(self) -> RatePerDay {
        RatePerDay(self.0 * 86400.0)
    }
}

impl From<f32> for RatePerSecond {
    fn from(v: f32) -> Self {
        RatePerSecond(v)
    }
}

impl From<RatePerSecond> for f32 {
    fn from(r: RatePerSecond) -> f32 {
        r.0
    }
}

impl Add for RatePerSecond {
    type Output = RatePerSecond;
    fn add(self, rhs: RatePerSecond) -> RatePerSecond {
        RatePerSecond(self.0 + rhs.0)
    }
}

impl Sub for RatePerSecond {
    type Output = RatePerSecond;
    fn sub(self, rhs: RatePerSecond) -> RatePerSecond {
        RatePerSecond(self.0 - rhs.0)
    }
}

impl Mul<f32> for RatePerSecond {
    type Output = RatePerSecond;
    fn mul(self, rhs: f32) -> RatePerSecond {
        RatePerSecond(self.0 * rhs)
    }
}

impl Div<f32> for RatePerSecond {
    type Output = RatePerSecond;
    fn div(self, rhs: f32) -> RatePerSecond {
        RatePerSecond(self.0 / rhs)
    }
}

impl fmt::Display for RatePerSecond {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.4} s⁻¹", self.0)
    }
}

/// Rate per day (1/day or day⁻¹)
/// Used for: drought rate evolution, weather system changes
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct RatePerDay(f32);

impl Eq for RatePerDay {}

impl PartialOrd for RatePerDay {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for RatePerDay {
    fn cmp(&self, other: &Self) -> Ordering {
        f32_total_cmp(self.0, other.0)
    }
}

impl Deref for RatePerDay {
    type Target = f32;
    #[inline]
    fn deref(&self) -> &f32 {
        &self.0
    }
}

impl DerefMut for RatePerDay {
    #[inline]
    fn deref_mut(&mut self) -> &mut f32 {
        &mut self.0
    }
}

impl RatePerDay {
    /// Seconds per day conversion factor
    pub const SECONDS_PER_DAY: f32 = 86400.0;

    /// Create a new rate per day (can be any value, positive or negative)
    #[inline]
    #[must_use]
    pub const fn new(value: f32) -> Self {
        RatePerDay(value)
    }

    /// Get the raw f32 value
    #[inline]
    #[must_use]
    pub fn value(self) -> f32 {
        self.0
    }

    /// Convert to rate per second
    #[inline]
    #[must_use]
    pub fn to_rate_per_second(self) -> RatePerSecond {
        RatePerSecond(self.0 / Self::SECONDS_PER_DAY)
    }
}

impl From<f32> for RatePerDay {
    fn from(v: f32) -> Self {
        RatePerDay(v)
    }
}

impl From<RatePerDay> for f32 {
    fn from(r: RatePerDay) -> f32 {
        r.0
    }
}

impl From<RatePerDay> for RatePerSecond {
    fn from(r: RatePerDay) -> RatePerSecond {
        r.to_rate_per_second()
    }
}

impl From<RatePerSecond> for RatePerDay {
    fn from(r: RatePerSecond) -> RatePerDay {
        r.to_rate_per_day()
    }
}

impl Add for RatePerDay {
    type Output = RatePerDay;
    fn add(self, rhs: RatePerDay) -> RatePerDay {
        RatePerDay(self.0 + rhs.0)
    }
}

impl Sub for RatePerDay {
    type Output = RatePerDay;
    fn sub(self, rhs: RatePerDay) -> RatePerDay {
        RatePerDay(self.0 - rhs.0)
    }
}

impl Mul<f32> for RatePerDay {
    type Output = RatePerDay;
    fn mul(self, rhs: f32) -> RatePerDay {
        RatePerDay(self.0 * rhs)
    }
}

impl Div<f32> for RatePerDay {
    type Output = RatePerDay;
    fn div(self, rhs: f32) -> RatePerDay {
        RatePerDay(self.0 / rhs)
    }
}

impl fmt::Display for RatePerDay {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.4} day⁻¹", self.0)
    }
}

// ============================================================================
// SURFACE AREA TO VOLUME RATIO
// ============================================================================

/// Surface area to volume ratio in m²/m³
/// Critical for fire spread calculations (Rothermel model)
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
#[repr(transparent)]
pub struct SurfaceAreaToVolume(f32);

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

impl Deref for SurfaceAreaToVolume {
    type Target = f32;
    #[inline]
    fn deref(&self) -> &f32 {
        &self.0
    }
}

impl DerefMut for SurfaceAreaToVolume {
    #[inline]
    fn deref_mut(&mut self) -> &mut f32 {
        &mut self.0
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

    /// Create a new surface area to volume ratio. Asserts value >= 0 (non-negative SAV).
    #[inline]
    #[must_use]
    pub const fn new(value: f32) -> Self {
        assert!(
            value >= 0.0,
            "SurfaceAreaToVolume::new: negative SAV is invalid"
        );
        SurfaceAreaToVolume(value)
    }

    /// Create without validation.
    /// # Safety
    /// Caller must ensure value >= 0 (non-negative SAV).
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(value: f32) -> Self {
        SurfaceAreaToVolume(value)
    }

    /// Get the raw f32 value
    #[inline]
    #[must_use]
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
    #[should_panic(expected = "Fraction::new: value not in [0, 1]")]
    fn test_fraction_new_panics_on_too_large() {
        let _ = Fraction::new(1.5);
    }

    #[test]
    #[should_panic(expected = "Fraction::new: value not in [0, 1]")]
    fn test_fraction_new_panics_on_negative() {
        let _ = Fraction::new(-0.5);
    }

    #[test]
    fn test_fraction_new_valid() {
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

    #[test]
    fn test_rate_per_second_arithmetic() {
        let r1 = RatePerSecond::new(0.5);
        let r2 = RatePerSecond::new(0.3);

        let sum = r1 + r2;
        assert!((sum.0 - 0.8).abs() < 0.01);

        let diff = r1 - r2;
        assert!((diff.0 - 0.2).abs() < 0.01);

        let scaled = r1 * 2.0;
        assert!((scaled.0 - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_rate_per_day_arithmetic() {
        let r1 = RatePerDay::new(2.0);
        let r2 = RatePerDay::new(1.5);

        let sum = r1 + r2;
        assert!((sum.0 - 3.5).abs() < 0.01);

        let diff = r1 - r2;
        assert!((diff.0 - 0.5).abs() < 0.01);

        let scaled = r1 / 2.0;
        assert!((scaled.0 - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_rate_per_day_to_rate_per_second() {
        let rpd = RatePerDay::new(86400.0);
        let rps = rpd.to_rate_per_second();
        assert!((rps.0 - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_rate_per_second_to_rate_per_day() {
        let rps = RatePerSecond::new(1.0);
        let rpd = rps.to_rate_per_day();
        assert!((rpd.0 - 86400.0).abs() < 0.01);
    }

    #[test]
    fn test_rate_conversions_bidirectional() {
        let original = RatePerSecond::new(0.001);
        let converted: RatePerDay = original.into();
        let back: RatePerSecond = converted.into();
        assert!((back.0 - original.0).abs() < 0.0001);
    }

    #[test]
    fn test_negative_rates_allowed() {
        let cooling = RatePerSecond::new(-0.05);
        assert!(cooling.0 < 0.0);

        let decline = RatePerDay::new(-2.5);
        assert!(decline.0 < 0.0);
    }
}
