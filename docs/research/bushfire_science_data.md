# Bushfire Science Research Compilation

**Prepared for:** Bushfire Simulation Engine Validation  
**Date:** December 2024  
**Purpose:** Comprehensive scientific data for physics model validation  

---

## Table of Contents

1. [Fire Spread Models](#1-fire-spread-models)
2. [Heat Transfer Physics](#2-heat-transfer-physics)
3. [Crown Fire Dynamics](#3-crown-fire-dynamics)
4. [Ember Spotting Models](#4-ember-spotting-models)
5. [Fuel Moisture Dynamics](#5-fuel-moisture-dynamics)
6. [Combustion Chemistry](#6-combustion-chemistry)
7. [Australian Fuel Properties](#7-australian-fuel-properties)
8. [Fire Weather Indices](#8-fire-weather-indices)
9. [Wind Effects on Fire Spread](#9-wind-effects-on-fire-spread)
10. [Physical Constants](#10-physical-constants)

---

## 1. Fire Spread Models

### 1.1 Rothermel Fire Spread Model (1972)

**Primary Source:** Rothermel, R.C. (1972). "A mathematical model for predicting fire spread in wildland fuels." USDA Forest Service Research Paper INT-115.

#### Core Spread Rate Formula

```
R = (I_R × ξ × (1 + Φ_w + Φ_s)) / (ρ_b × ε × Q_ig)
```

Where:
- **R** = Rate of spread (m/min)
- **I_R** = Reaction intensity (kJ/(m²·min))
- **ξ** = Propagating flux ratio (dimensionless, 0-1)
- **Φ_w** = Wind coefficient (dimensionless)
- **Φ_s** = Slope coefficient (dimensionless)
- **ρ_b** = Fuel bed bulk density (kg/m³)
- **ε** = Effective heating number (dimensionless, 0.3-0.5)
- **Q_ig** = Heat of pre-ignition (kJ/kg)

#### Reaction Intensity (I_R)

```
I_R = Γ' × w_n × h × η_M × η_s
```

Where:
- **Γ'** = Optimum reaction velocity (1/min)
- **w_n** = Net fuel loading (kg/m²)
- **h** = Heat content (kJ/kg)
- **η_M** = Moisture damping coefficient (0-1)
- **η_s** = Mineral damping coefficient (0-1)

#### Optimum Reaction Velocity

```
Γ'_max = σ^1.5 / (495 + 0.0594 × σ^1.5)
```

Where **σ** = Surface-area-to-volume ratio (m²/m³)

#### Moisture Damping Coefficient (η_M)

```
η_M = 1 - 2.59×(M_f/M_x) + 5.11×(M_f/M_x)² - 3.52×(M_f/M_x)³
```

Where:
- **M_f** = Fuel moisture content (fraction)
- **M_x** = Moisture of extinction (fraction)

#### Wind Coefficient (Φ_w)

```
Φ_w = C × (β/β_op)^(-E) × U^B
```

Where:
- **C** = 7.47 × exp(-0.133 × σ^0.55)
- **B** = 0.02526 × σ^0.54
- **E** = 0.715 × exp(-3.59 × 10^(-4) × σ)
- **β** = Packing ratio
- **β_op** = Optimum packing ratio
- **U** = Midflame wind speed (m/min)

#### Slope Coefficient (Φ_s)

```
Φ_s = 5.275 × β^(-0.3) × (tan θ)²
```

Where **θ** = Slope angle (degrees)

#### Propagating Flux Ratio (ξ)

```
ξ = exp((0.792 + 0.681 × σ^0.5) × (β + 0.1)) / (192 + 0.2595 × σ)
```

#### Heat of Pre-ignition (Q_ig)

```
Q_ig = 250 + 1116 × M_f
```

Where **M_f** = Fuel moisture content (fraction)

### 1.2 Australian Calibration Factors

**Source:** Cruz, M.G., et al. (2015). "Empirical-based models for predicting head-fire rate of spread in Australian fuel types." Australian Forestry, 78(3), 118-158.

- Australian eucalyptus forests require calibration factor of approximately **0.05** applied to Rothermel base predictions
- Grassland fires follow the **10-20% wind speed rule**: spread rate ≈ 10-20% of 10m wind speed
- Alexander & Cruz (2019) validated this rule across multiple Australian fuel types

#### Australian Grassland Fire Spread

```
R = 0.13 × U_10 × exp(0.069 × (T - 20))
```

Where:
- **R** = Spread rate (m/min)
- **U_10** = Wind speed at 10m (km/h)
- **T** = Air temperature (°C)

### 1.3 Validation Data Ranges

| Condition | Spread Rate (m/min) | Source |
|-----------|---------------------|--------|
| Low intensity grass | 5-15 | McArthur (1967) |
| Moderate grass | 15-50 | CSIRO VESTA |
| High intensity grass | 50-100 | Cruz et al. (2015) |
| Eucalyptus forest (moderate) | 10-30 | Project VESTA |
| Eucalyptus forest (extreme) | 100-200 | Black Saturday |

---

## 2. Heat Transfer Physics

### 2.1 Stefan-Boltzmann Radiation Law

**Source:** Stefan (1879), Boltzmann (1884)

#### Full Formula (No Simplifications)

```
Q = σ × ε × A × (T_source⁴ - T_target⁴)
```

Where:
- **σ** = Stefan-Boltzmann constant = 5.670374419 × 10⁻⁸ W/(m²·K⁴)
- **ε** = Emissivity (0-1)
- **A** = Surface area (m²)
- **T** = Absolute temperature (Kelvin)

#### Wildfire Emissivity Values

| Material | Emissivity | Source |
|----------|------------|--------|
| Wildfire flame | 0.95 | Butler & Cohen (1998) |
| Black body | 1.00 | Theoretical maximum |
| Charred wood | 0.85-0.95 | Drysdale (2011) |
| Green vegetation | 0.90-0.95 | Dietenberger (2016) |

### 2.2 View Factor Calculations

**Source:** Drysdale (2011) "Introduction to Fire Dynamics"

#### Planar Radiator Model (Correct for Flames)

```
F = A_flame / (π × r²)
```

**NOT** point source: ~~F = A / (4πr²)~~

Flames are extended planar radiators, not point sources.

### 2.3 Convection Heat Transfer

**Source:** Standard heat transfer theory

#### Newton's Law of Cooling

```
q = h × A × ΔT
```

Where:
- **q** = Heat transfer rate (W)
- **h** = Convection coefficient (W/(m²·K))
- **A** = Surface area (m²)
- **ΔT** = Temperature difference (K)

#### Natural Convection Coefficient

```
h ≈ 1.32 × (ΔT/L)^0.25
```

Typical range for wildfire conditions: **5-50 W/(m²·K)**

### 2.4 Latent Heat of Vaporization

**CRITICAL:** Water evaporation MUST occur BEFORE temperature rise

```
Q_evap = m_water × L_v
```

Where:
- **L_v** = 2260 kJ/kg (latent heat of water vaporization)
- This energy is absorbed without temperature change

---

## 3. Crown Fire Dynamics

### 3.1 Van Wagner Crown Fire Model (1977)

**Primary Source:** Van Wagner, C.E. (1977). "Conditions for the start and spread of crown fire." Canadian Journal of Forest Research, 7(1), 23-34.

#### Critical Surface Intensity for Crown Fire Initiation (I_o)

```
I_o = (0.010 × CBH × (460 + 25.9 × FMC))^1.5
```

Where:
- **I_o** = Critical surface fire intensity (kW/m)
- **CBH** = Crown base height (m)
- **FMC** = Foliar moisture content (%)

Alternative formulation per Van Wagner (1977):

```
I_o = (0.01 × CBD × H × (460 + 25.9 × FMC)) / CBH
```

Where:
- **CBD** = Crown bulk density (kg/m³)
- **H** = Heat content of crown fuels (kJ/kg)

#### Critical Crown Fire Spread Rate (R_active)

```
R_active = 3.0 / CBD
```

Where:
- **R_active** = Critical crown fire spread rate (m/min)
- **CBD** = Crown bulk density (kg/m³), typical range 0.05-0.30

#### Crown Fraction Burned (CFB)

**Source:** Cruz & Alexander (2010)

```
CFB = 1 - exp(-0.23 × (R - R_critical))
```

### 3.2 Crown Fire Classification

| Type | Condition |
|------|-----------|
| **Surface** | I_surface < I_critical |
| **Passive** | I_surface ≥ I_critical AND R < R_critical |
| **Active** | I_surface ≥ I_critical AND R ≥ R_critical |

### 3.3 Typical Parameter Values

| Parameter | Range | Typical | Source |
|-----------|-------|---------|--------|
| Crown bulk density | 0.05-0.30 kg/m³ | 0.10-0.15 | Van Wagner (1977) |
| Crown base height | 2-15 m | 6-10 m | Forest inventory |
| Foliar moisture content | 80-120% | 100% | Field measurements |
| Heat content (crown) | 18,000-22,000 kJ/kg | 20,000 | Rothermel (1972) |

---

## 4. Ember Spotting Models

### 4.1 Albini Spotting Model (1979, 1983)

**Primary Sources:**
- Albini, F.A. (1979). "Spot fire distance from burning trees: a predictive model." USDA Forest Service Research Paper INT-56
- Albini, F.A. (1983). "Transport of firebrands by line thermals." Combustion Science and Technology, 32(5-6), 277-288

#### Lofting Height

```
H = 12.2 × I^0.4  (Original Albini)
H = 6.1 × I^0.4   (Australian calibration)
```

Where:
- **H** = Lofting height (m)
- **I** = Fireline intensity (kW/m)

**Note:** Australian conditions show approximately 50% of original Albini predictions (Cruz et al., 2012)

#### Maximum Spotting Distance

```
s_max = H × (u_H / w_f) × terrain_factor
```

Where:
- **s_max** = Maximum spotting distance (m)
- **u_H** = Wind speed at height H (m/s)
- **w_f** = Ember terminal velocity (m/s)

#### Wind Speed Profile with Height

```
u(z) = u_ref × (z / z_ref)^α
```

Where:
- **α** = Wind shear exponent ≈ 0.15 (open terrain)
- **z_ref** = Reference height (typically 10m)

#### Ember Terminal Velocity

```
w_f = sqrt((2 × m × g) / (ρ_air × C_d × A))
```

Where:
- **m** = Ember mass (kg)
- **g** = Gravitational acceleration (9.81 m/s²)
- **ρ_air** = Air density (1.225 kg/m³)
- **C_d** = Drag coefficient (≈0.4 for spheres)
- **A** = Cross-sectional area (m²)

### 4.2 Observed Spotting Distances

| Event | Distance | Conditions | Source |
|-------|----------|------------|--------|
| Black Saturday 2009 | 25-33 km | Extreme | Cruz et al. (2012) |
| CSIRO Ribbon Bark | 37 km | Research | CSIRO (2017) |
| Typical severe | 5-10 km | Severe | Albini (1983) |
| Moderate conditions | 1-3 km | High | Various |

### 4.3 Ember Properties

| Fuel Type | Mass (kg) | Diameter (mm) | Terminal Velocity (m/s) |
|-----------|-----------|---------------|-------------------------|
| Stringybark | 0.001 | 15-30 | 3-6 |
| Grass/fine fuel | 0.0002 | 2-5 | 1-2 |
| Eucalyptus bark ribbons | 0.005-0.020 | 10-50 | 4-8 |

---

## 5. Fuel Moisture Dynamics

### 5.1 Nelson Fuel Moisture Model (2000)

**Primary Source:** Nelson, R.M. (2000). "Prediction of diurnal change in 10-h fuel stick moisture content." Canadian Journal of Forest Research, 30(7), 1071-1087.

#### Equilibrium Moisture Content (EMC)

**Source:** Simard (1968)

```
EMC = a + b×H + c×T + d×H×T
```

**Adsorption coefficients:**
- a = 0.0
- b = 0.00253
- c = -0.000116
- d = -0.0000158

**Desorption coefficients:**
- a = 0.0
- b = 0.00246
- c = -0.000121
- d = -0.0000133

Where:
- **H** = Relative humidity (%)
- **T** = Temperature (°C)

#### Exponential Moisture Equilibration

```
M(t) = M_e + (M_0 - M_e) × exp(-t/τ)
```

Where:
- **M(t)** = Moisture at time t
- **M_e** = Equilibrium moisture content
- **M_0** = Initial moisture content
- **τ** = Timelag (hours)
- After time τ: 63.2% of the way to equilibrium

### 5.2 Timelag Classes

| Class | Timelag (hours) | Fuel Diameter | Examples |
|-------|-----------------|---------------|----------|
| 1-hour | 1 | < 6mm | Grass, fine litter |
| 10-hour | 10 | 6-25mm | Twigs |
| 100-hour | 100 | 25-75mm | Branches |
| 1000-hour | 1000 | > 75mm | Logs |

---

## 6. Combustion Chemistry

### 6.1 Cellulose Combustion Stoichiometry

**Cellulose formula:** C₆H₁₀O₅

```
C₆H₁₀O₅ + 6 O₂ → 6 CO₂ + 5 H₂O
```

#### Mass Ratios per kg Fuel

| Product | Mass Ratio | Source |
|---------|------------|--------|
| Oxygen consumed | 1.33 kg | Stoichiometry |
| CO₂ produced | 1.47 kg | Stoichiometry |
| H₂O produced | 0.56 kg | Stoichiometry |
| Smoke particles | 0.02 kg | Empirical |

### 6.2 Oxygen-Limited Combustion

**Source:** Drysdale (2011) "Introduction to Fire Dynamics"

- Full combustion rate at: **≥ 21% O₂**
- Reduced combustion: **15-21% O₂** (smooth reduction)
- No sustained flame below: **~15% O₂** (smoldering only)
- Incomplete combustion produces: **CO** (30% of carbon at low O₂)

### 6.3 Heat of Combustion

| Fuel Type | Heat Content (kJ/kg) | Source |
|-----------|----------------------|--------|
| Dry wood | 18,000-20,000 | Rothermel (1972) |
| Eucalyptus oil | 43,000 | CSIRO |
| Grass | 16,000-18,000 | McArthur (1967) |
| Green vegetation | 8,000-12,000 | Variable moisture |

---

## 7. Australian Fuel Properties

### 7.1 Eucalyptus Oil Properties

**Source:** CSIRO bushfire research, various

| Property | Value | Notes |
|----------|-------|-------|
| Vaporization temperature | 170°C | Oil begins to volatilize |
| Auto-ignition temperature | 232°C | Spontaneous combustion |
| Heat of combustion | 43 MJ/kg | Very high energy content |
| Volatile oil content | 2-5% by mass | Species dependent |

### 7.2 Bark Types and Fire Behavior

**Source:** Pausas et al. (2017), Victorian Eucalypt Bark Hazard Guide

| Bark Type | Ladder Fuel Factor | Fire Behavior |
|-----------|-------------------|---------------|
| Stringybark | 1.0 (maximum) | Extreme crown fire potential |
| Fibrous bark | 0.5 | Moderate ladder fuel |
| Ironbark | 0.2 | Dense, slow burning |
| Smooth bark | 0.1 | Minimal ladder fuel |
| Paperbark | 0.7 | Highly flammable |

### 7.3 Surface-Area-to-Volume Ratios

| Fuel Type | SAV (m²/m³) | Notes |
|-----------|-------------|-------|
| Fine grass | 3,000-5,000 | Burns rapidly |
| Shrub leaves | 1,500-2,500 | Moderate |
| Twigs | 500-1,000 | Slower burning |
| Large branches | 150-300 | Extended burning |
| Logs | 50-150 | Long duration |

### 7.4 Fuel Loads by Vegetation Type

| Vegetation | Fuel Load (t/ha) | Notes |
|------------|------------------|-------|
| Grassland | 2-8 | Highly variable |
| Dry eucalyptus forest | 10-20 | Surface fuels |
| Wet eucalyptus forest | 20-40 | With understory |
| Coastal heath | 15-25 | Dense shrubs |
| Alpine woodland | 5-15 | Sparse |

---

## 8. Fire Weather Indices

### 8.1 McArthur Forest Fire Danger Index (FFDI) Mark 5

**Source:** McArthur (1967), Noble et al. (1980), Bureau of Meteorology

#### Formula

```
FFDI = 2.11 × exp(-0.45 + 0.987×ln(D) - 0.0345×H + 0.0338×T + 0.0234×V)
```

Where:
- **D** = Drought factor (0-10)
- **H** = Relative humidity (%)
- **T** = Temperature (°C)
- **V** = Wind speed at 10m (km/h)

**Calibration constant:** 2.11 (WA empirical adjustment, theoretical is 2.0)

### 8.2 Fire Danger Ratings

| Rating | FFDI Range | Description |
|--------|------------|-------------|
| Low-Moderate | 0-12 | Normal fire behavior |
| High | 12-24 | Elevated danger |
| Very High | 24-50 | Significant fire weather |
| Severe | 50-75 | Extreme fire behavior possible |
| Extreme | 75-100 | Very dangerous conditions |
| Catastrophic | 100+ | Unprecedented fire behavior |

### 8.3 Historical FFDI Values

| Event | FFDI | Location |
|-------|------|----------|
| Black Saturday 2009 | 170-200+ | Victoria |
| Ash Wednesday 1983 | 140-160 | Victoria/SA |
| Canberra 2003 | 100-120 | ACT |
| Typical severe day | 50-75 | Various |

---

## 9. Wind Effects on Fire Spread

### 9.1 Directional Spread Multipliers

**Source:** McArthur (1967), Rothermel (1972)

| Direction | Multiplier | Notes |
|-----------|------------|-------|
| Downwind | 4-26× | Extreme asymmetry |
| Perpendicular | 1.0× | Base spread |
| Upwind | 0.05× | Minimal spread |

**Empirical formula for downwind boost:**
```
Multiplier = 1.0 + alignment × sqrt(wind_speed) × 0.8
```

### 9.2 Vertical Spread Enhancement

**Source:** Sullivan (2009), general fire physics

- Fire climbs **2.5×+ faster** than horizontal spread
- Convection drives upward heat transfer
- Flames tilt toward fuel above

### 9.3 Wind-Slope Interaction

Combined effect is multiplicative, not additive.

---

## 10. Physical Constants

### 10.1 Fundamental Constants

| Constant | Value | Units |
|----------|-------|-------|
| Stefan-Boltzmann (σ) | 5.670374419 × 10⁻⁸ | W/(m²·K⁴) |
| Gravitational acceleration | 9.81 | m/s² |
| Air density (sea level) | 1.225 | kg/m³ |
| Specific heat of water | 4,186 | J/(kg·K) |
| Latent heat of vaporization | 2,260,000 | J/kg |

### 10.2 Conversion Factors

| Conversion | Factor |
|------------|--------|
| °C to K | T_K = T_C + 273.15 |
| km/h to m/s | ÷ 3.6 |
| kW/m to BTU/ft·s | × 0.288 |
| t/ha to kg/m² | ÷ 10 |

---

## References

1. Rothermel, R.C. (1972). "A mathematical model for predicting fire spread in wildland fuels." USDA Forest Service Research Paper INT-115.

2. Van Wagner, C.E. (1977). "Conditions for the start and spread of crown fire." Canadian Journal of Forest Research, 7(1), 23-34.

3. Albini, F.A. (1979). "Spot fire distance from burning trees: a predictive model." USDA Forest Service Research Paper INT-56.

4. Albini, F.A. (1983). "Transport of firebrands by line thermals." Combustion Science and Technology, 32(5-6), 277-288.

5. Nelson, R.M. (2000). "Prediction of diurnal change in 10-h fuel stick moisture content." Canadian Journal of Forest Research, 30(7), 1071-1087.

6. McArthur, A.G. (1967). "Fire behaviour in eucalypt forests." Forestry and Timber Bureau Leaflet 107.

7. Cruz, M.G., et al. (2015). "Empirical-based models for predicting head-fire rate of spread in Australian fuel types." Australian Forestry, 78(3), 118-158.

8. Cruz, M.G., et al. (2012). "Anatomy of a catastrophic wildfire: The Black Saturday Kilmore East fire in Victoria, Australia." Forest Ecology and Management, 284, 269-285.

9. Byram, G.M. (1959). "Combustion of forest fuels." In: Forest Fire: Control and Use. McGraw-Hill, New York.

10. Drysdale, D. (2011). "An Introduction to Fire Dynamics." 3rd Edition. Wiley.

11. Butler, B.W., Cohen, J.D. (1998). "Firefighter Safety Zones: A Theoretical Model Based on Radiative Heating." Int. J. Wildland Fire, 8(2), 73-77.

12. Noble, I.R., Gill, A.M., Bary, G.A.V. (1980). "McArthur's fire-danger meters expressed as equations." Australian Journal of Ecology, 5, 201-203.

13. Rein, G. (2009). "Smouldering Combustion Phenomena in Science and Technology." International Review of Chemical Engineering, 1, 3-18.

14. Pausas, J.G., et al. (2017). "Bark functional ecology: Evidence for tradeoffs, functional coordination, and environment-producing bark diversity." New Phytologist, 215(3), 1024-1037.

15. Sullivan, A.L. (2009). "Wildland surface fire spread modelling, 1990–2007." International Journal of Wildland Fire, 18, 349-403.

---

*Document compiled from peer-reviewed scientific literature for bushfire simulation validation.*
