# Comprehensive Catalog of Bushfire/Wildfire Behaviors

**Purpose:** Complete reference of all physical phenomena, behaviors, and effects that occur during bushfires. Used for auditing fire simulation completeness.

**Sources:** USDA Forest Service, CSIRO, Bureau of Meteorology Australia, NWCG, Natural Hazards Research Australia, peer-reviewed fire science literature.

---

## Table of Contents

1. [Fire Spread Mechanisms](#1-fire-spread-mechanisms)
2. [Terrain Effects](#2-terrain-effects)
3. [Weather Interactions](#3-weather-interactions)
4. [Fuel Behaviors](#4-fuel-behaviors)
5. [Combustion Phases](#5-combustion-phases)
6. [Fire Intensity Effects](#6-fire-intensity-effects)
7. [Extreme Fire Behaviors](#7-extreme-fire-behaviors)
8. [Secondary Effects](#8-secondary-effects)
9. [Fire Geometry and Spread Patterns](#9-fire-geometry-and-spread-patterns)

---

## 1. Fire Spread Mechanisms

### 1.1 Surface Fire Spread

**Description:** Fire burning through surface fuels (litter, grass, shrubs) on the ground. The most common fire type and foundation for crown fire development.

**Scientific Basis:**
- Heat transfer via radiation, convection, and conduction preheats unburned fuel
- Fuel ahead of flame front is dried, raising temperature to ignition point (~300°C for wood)
- Rate of spread (ROS) depends on fuel, moisture, wind, and slope

**Key Parameters:**
- Fuel load (kg/m²)
- Fuel moisture content (%)
- Surface area-to-volume ratio (m⁻¹)
- Packing ratio (dimensionless)
- Fuel bed depth (m)
- Wind speed at midflame height (m/s)
- Slope angle (degrees)

**Primary Model:** Rothermel (1972) - USDA INT-115
```
R = (I_R × ξ × (1 + φ_w + φ_s)) / (ρ_b × ε × Q_ig)

Where:
R = rate of spread (m/min)
I_R = reaction intensity (kW/m²)
ξ = propagating flux ratio
φ_w = wind factor
φ_s = slope factor
ρ_b = bulk density (kg/m³)
ε = effective heating number
Q_ig = heat of preignition (kJ/kg)
```

**Australian Considerations:**
- McArthur models developed specifically for Australian eucalypt forests and grasslands
- Different fuel classification systems (Overall Fuel Hazard Assessment)
- Stringybark forests create exceptional surface fuel continuity

---

### 1.2 Crown Fire Spread

**Description:** Fire spreading through the forest canopy, either dependent on (passive) or independent of (active) surface fire.

**Crown Fire Types:**

| Type | Description | Behavior |
|------|-------------|----------|
| **Passive Crown Fire** | Individual tree torching; crown fire depends on surface fire | Intermittent crowning |
| **Active Crown Fire** | Continuous crown fire that sustains itself | Crown fire spreads with surface fire |
| **Independent Crown Fire** | Crown fire spreads without surface fire support | Rare; extreme conditions |

**Scientific Basis:**
- Surface fire intensity must exceed critical threshold to ignite crowns
- Vertical heat flux must be sufficient to bridge gap between surface and crown fuels
- Ladder fuels (intermediate vegetation) lower ignition threshold
- Sustained crown fire requires minimum canopy bulk density

**Key Parameters:**
- Crown base height (m)
- Canopy bulk density (kg/m³)
- Foliar moisture content (%)
- Surface fireline intensity (kW/m)
- Wind speed (km/h)

**Primary Model:** Van Wagner (1977)
```
Crown Fire Initiation:
I_0 = (0.010 × CBH × (460 + 25.9 × FMC))^(3/2)

Where:
I_0 = critical surface fire intensity for crown ignition (kW/m)
CBH = crown base height (m)
FMC = foliar moisture content (%)

Active Crown Fire Threshold:
R_active = 3.0 / CBD

Where:
CBD = canopy bulk density (kg/m³)
R_active = minimum ROS for sustained crown fire (m/min)
```

**Australian Considerations:**
- Eucalyptus crowns often 20-40m above ground
- Oil-rich foliage extremely flammable (43 MJ/kg)
- Stringybark creates natural ladder fuels
- Crown fires common during severe fire weather (FFDI > 50)

---

### 1.3 Spotting / Ember Transport

**Description:** Firebrands (burning material) lofted by fire plume, transported by wind, landing ahead of fire front to ignite new fires.

**Three-Phase Process:**
1. **Generation:** Bark, leaves, twigs torn from burning vegetation
2. **Transport:** Lofted by convection, carried by ambient wind
3. **Ignition:** Firebrand lands on receptive fuel, ignites spot fire

**Key Parameters:**
- Firebrand size, shape, density
- Initial combustion state
- Plume height and velocity
- Ambient wind speed/direction
- Transport distance and time
- Recipient fuel moisture and type

**Primary Model:** Albini (1979, 1983)
```
Maximum Spotting Distance (flat terrain):
z_max = 12.2 × H_F  (maximum loft height)

Horizontal transport limited by:
- Burnout time of firebrand
- Terminal velocity
- Wind speed profile

Spot fire ignition probability function of:
- Firebrand energy at landing
- Recipient fuel moisture
- Fuel bed characteristics
```

**Australian Considerations:**
- **Ribbon bark eucalypts** (E. viminalis, E. rubida) notorious for long-distance spotting
- Bark strips can burn for 30+ minutes at terminal velocity
- **Black Saturday 2009:** Spotting distances up to 33 km documented
- Typical long-range spotting: 5-15 km common in severe conditions
- Mass spotting (ember storms) during plume collapse

**Spotting Distance by Bark Type:**
| Bark Type | Typical Distance | Maximum Observed |
|-----------|-----------------|------------------|
| Stringybark | 1-3 km | 5 km |
| Ribbon bark | 5-15 km | 33 km |
| Messmate | 0.5-2 km | 4 km |
| Ironbark | 0.1-0.5 km | 1 km |

---

### 1.4 Fire Runs

**Description:** Sudden acceleration of fire spread, often associated with changes in fuel, terrain, or weather.

**Causes:**
- Wind increase or direction change
- Fire reaching steeper slopes
- Fuel transition to more flammable type
- Plume development enhancing indraft
- Junction of multiple fire fronts

**Characteristics:**
- Spread rate may increase 5-10x normal
- Duration: minutes to hours
- Often followed by intensity decrease when conditions change

**Key Parameters:**
- Initial spread rate
- Triggering event characteristics
- Duration of accelerated spread
- Maximum spread rate achieved

---

### 1.5 Fire Whirls / Fire Tornadoes

**Description:** Rotating columns of fire and air, ranging from small (10-50m tall) to large fire tornadoes (EF0-EF2 equivalent).

**Formation Conditions:**
1. Intense heat release creating strong updraft
2. Horizontal vorticity (from wind shear or terrain)
3. Tilting of horizontal vorticity into vertical
4. Vortex stretching as air accelerates upward

**Types:**

| Type | Height | Diameter | Wind Speed | Duration |
|------|--------|----------|------------|----------|
| Small whirl | 10-50m | 1-5m | 10-40 km/h | Seconds-minutes |
| Medium whirl | 50-200m | 5-30m | 40-120 km/h | Minutes |
| Fire tornado | 200m+ | 30-100m+ | 120-250 km/h | Minutes-hours |

**Scientific Basis:**
- Vorticity equation: ∂ω/∂t = (ω·∇)v - ω(∇·v) + (1/ρ²)(∇ρ × ∇p) + ν∇²ω
- Angular momentum conservation as radius decreases
- Centripetal acceleration balanced by pressure gradient

**Key Parameters:**
- Fire intensity and heat release rate
- Ambient wind shear (vertical and horizontal)
- Atmospheric stability
- Terrain-induced vorticity

**Notable Examples:**
- **Carr Fire (2018):** EF-3 equivalent fire tornado, 230 km/h winds
- **Canberra Firestorm (2003):** Multiple fire whirls during extreme conditions
- **Great Kanto Earthquake (1923):** Fire whirl killed 38,000 people

---

## 2. Terrain Effects

### 2.1 Slope Effects

**Description:** Fire spreads faster uphill, slower downhill due to flame geometry and convective preheating.

**Scientific Basis:**
- Flames lean toward uphill fuel, increasing radiant heat transfer
- Convective preheating more effective uphill
- Effective wind equivalent: each 10° slope ≈ additional wind speed

**Slope Factor (Rothermel):**
```
φ_s = 5.275 × β^(-0.3) × tan²(θ)

Where:
φ_s = slope factor
β = packing ratio
θ = slope angle

Rule of Thumb:
- Rate of spread doubles for each 10° slope increase
- 20° slope = 4x flat-ground spread
- 30° slope = 8x flat-ground spread
```

**Key Parameters:**
- Slope angle (degrees)
- Slope aspect
- Fire approach direction relative to slope

---

### 2.2 Valley Channeling (Chimney Effect)

**Description:** Rapid fire acceleration in steep V-shaped valleys, canyons, or gullies due to terrain-induced wind acceleration and convective enhancement.

**Scientific Basis:**
- Valley geometry focuses convective heating
- Wind accelerates through constriction (venturi effect)
- Fire-induced indrafts amplified by terrain shape
- Concave terrain enhances radiant heat concentration

**Eruptive Fire Behavior:**
- Sudden transition from steady to explosive spread
- Spread rates can exceed 10 km/h
- Associated with many firefighter fatalities

**Key Parameters:**
- Valley width and depth
- Slope angle of valley walls
- Valley orientation relative to wind
- Fuel continuity along valley floor

**Formula (empirical):**
```
Fire acceleration in canyon:
ROS_canyon = ROS_flat × K_canyon

Where K_canyon depends on:
- Valley slope angle
- Valley width-to-depth ratio
- Wind alignment with valley axis
```

**Critical Safety Consideration:**
Canyons, chutes, saddles, and narrow drainages are known as "Watch Out Situations" in firefighting due to potential for rapid fire acceleration.

---

### 2.3 Aspect Effects

**Description:** Slope orientation (aspect) affects fuel moisture, temperature, and fire behavior.

**Northern vs Southern Hemisphere:**

| Aspect | Southern Hemisphere | Northern Hemisphere |
|--------|--------------------|--------------------|
| North-facing | More sun, drier fuels, higher fire risk | Less sun, moister fuels, lower risk |
| South-facing | Less sun, moister fuels, lower risk | More sun, drier fuels, higher risk |
| West-facing | Afternoon heating, evening fire activity | Afternoon heating, evening fire activity |
| East-facing | Morning warming, moderate fire potential | Morning warming, moderate fire potential |

**Key Parameters:**
- Solar radiation received (W/m²)
- Fuel moisture differential (1-5% variation)
- Vegetation type differences
- Local wind patterns

---

### 2.4 Ridge Behavior

**Description:** Fire behavior changes significantly near ridgetops due to wind patterns and terrain transitions.

**Effects:**
- Wind acceleration over ridges
- Turbulent eddies on lee side
- Fire may slow briefly at ridge then accelerate descending
- Ridge-top fires create bidirectional spotting

**Key Parameters:**
- Ridge sharpness/geometry
- Wind speed and direction
- Fire approach angle

---

### 2.5 Saddle/Gap Effects

**Description:** Terrain saddles, gaps, and passes channel wind and funnel fire.

**Characteristics:**
- Wind acceleration through gaps
- Fire funneling effect
- Enhanced spotting across gaps
- Rapid fire spread through saddles

---

### 2.6 Vorticity-Driven Lateral Spread (VLS)

**Description:** Rapid lateral fire spread across steep leeward slopes, perpendicular to the prevailing wind.

**Scientific Basis:**
- Wind flow separation creates horizontal vortices on lee slopes
- Fire induces additional buoyancy-driven vorticity
- Coupled vorticity drives lateral spread along ridgeline
- Can cause fire to "jump" across terrain features

**Conditions for VLS:**
1. Slope angle > 20°
2. Wind speed > 20 km/h
3. Wind direction perpendicular to slope
4. Leeward (downwind) slope exposure

**Key Parameters:**
- Slope angle
- Wind speed at ridgetop
- Wind-slope alignment angle
- Fuel continuity

**Australian Significance:**
- VLS identified as major factor in 2003 Canberra fires
- Caused rapid fire spread in unexpected directions
- Often precursor to extreme fire behavior

---

## 3. Weather Interactions

### 3.1 Wind Effects on Spread

**Description:** Wind is the primary driver of fire spread rate and direction.

**Effects:**
- Tilts flames toward unburned fuel, increasing radiant preheating
- Increases oxygen supply to combustion zone
- Transports embers for spotting
- Creates asymmetric fire shape (elliptical)

**Wind Speed Relationships:**
```
Rothermel Wind Factor:
φ_w = C × (3.281 × U)^B × (β/β_op)^(-E)

Where:
U = midflame wind speed (m/s)
C, B, E = fuel-specific constants
β = packing ratio
β_op = optimum packing ratio

General relationship:
- Doubling wind speed approximately doubles spread rate
- Effect nonlinear at high wind speeds (>40 km/h)
```

**Head Fire vs Backing Fire:**
| Fire Type | Direction | Relative ROS |
|-----------|-----------|--------------|
| Head fire | With wind | 1.0 (reference) |
| Flank fire | Perpendicular | 0.1-0.3 |
| Backing fire | Against wind | 0.05-0.1 |

---

### 3.2 Wind Shifts and Cold Front Effects

**Description:** Sudden changes in wind direction, often associated with cold fronts, create dangerous fire behavior.

**Effects:**
- Fire flank becomes new head fire
- Greatly expanded fire perimeter
- Rapid intensity increase
- Changed spotting direction

**Cold Front Passage:**
1. Wind shifts 45°-180° with frontal passage
2. Brief wind lull possible before shift
3. Wind often increases after frontal passage
4. Relative humidity may temporarily increase then decrease

**Key Parameters:**
- Pre-frontal wind direction and speed
- Post-frontal wind direction and speed
- Rate of wind change
- Timing relative to fire position

**Australian Significance:**
- Many major fire disasters associated with cold front wind changes
- Black Saturday (2009): Extreme NW winds shifted to SW
- Ash Wednesday (1983): Similar pattern

---

### 3.3 Humidity Effects

**Description:** Relative humidity directly affects fuel moisture and fire behavior.

**Effects:**
- Low RH → dry fuels → easier ignition, faster spread
- High RH → moist fuels → slower spread, reduced intensity
- RH < 20%: Critical fire weather
- RH < 10%: Extreme fire weather

**Key Parameters:**
- Relative humidity (%)
- Temperature (affects equilibrium moisture content)
- Time of exposure (fuel response time)

---

### 3.4 Temperature Effects

**Description:** Ambient temperature affects fuel moisture, ignition, and fire behavior.

**Effects:**
- Higher temperature → lower fuel moisture
- Higher temperature → faster drying
- Higher temperature → earlier ignition
- Air density effects on convection

**Key Parameters:**
- Air temperature (°C)
- Fuel temperature (may exceed air temperature in sun)
- Dew point temperature

---

### 3.5 Atmospheric Stability

**Description:** Vertical temperature profile affects plume development and fire behavior.

**Stability Categories:**

| Stability | Lapse Rate | Fire Behavior |
|-----------|-----------|---------------|
| **Unstable** | > 9.8°C/km | Strong convection, tall plumes, erratic behavior |
| **Neutral** | ≈ 9.8°C/km | Normal fire development |
| **Stable** | < 9.8°C/km | Suppressed convection, smoke layering |

**Haines Index:** Lower atmosphere stability and dryness index
- Calculated from 850-700 mb layer
- Scale 2-6, with 6 indicating highest fire growth potential
- Predicts large fire growth potential

**Key Parameters:**
- Temperature lapse rate
- Mixing height
- Inversion presence/strength

---

### 3.6 Pyroconvection (Fire-Generated Weather)

**Description:** Intense fires generate their own atmospheric convection, affecting fire behavior.

**Stages:**
1. **Smoke plume:** Simple buoyant rise, minimal weather effects
2. **Pyrocumulus (pyroCu):** Convective cloud forms above fire
3. **Pyrocumulonimbus (pyroCb):** Deep convection, thunderstorm-like behavior

**Effects:**
- Enhanced updrafts (measured up to 50+ m/s)
- Fire-generated winds (indrafts)
- Erratic and unpredictable behavior
- Long-range spotting

---

### 3.7 Pyrocumulonimbus (pyroCb) Clouds

**Description:** Fire-generated thunderstorm clouds reaching the upper troposphere or lower stratosphere.

**Formation Requirements:**
- High fire intensity (>10,000 kW/m typically)
- Unstable atmosphere
- Sufficient moisture
- Sustained heat release

**Associated Phenomena:**
- **Lightning:** Can start new fires (dry lightning)
- **Downdrafts:** Erratic surface winds up to 100+ km/h
- **Enhanced spotting:** Plume lofts embers to great heights
- **Precipitation:** Rarely sufficient to suppress fire
- **Hail:** Possible in intense pyroCb

**Key Parameters:**
- Fire heat release rate (MW)
- Atmospheric instability (CAPE)
- Mid-level moisture
- Wind shear profile

**Australian Significance:**
- Black Summer 2019-20: 18 pyroCb events documented
- Stratospheric smoke injection affects climate
- Associated with most extreme fire behavior

---

### 3.8 Downdrafts from Pyroconvection

**Description:** Cold air descending from pyroconvective cloud causes sudden, dangerous wind changes.

**Characteristics:**
- Wind speeds can exceed 100 km/h
- Direction may be any orientation relative to ambient wind
- Duration: minutes to tens of minutes
- Accompanied by smoke and sometimes ember fallout

**Effects on Fire:**
- Sudden fire acceleration
- Unpredictable direction changes
- Enhanced spotting
- Flattened flame angle increasing radiant heating

---

### 3.9 Diurnal Weather Patterns

**Description:** Day-night cycles in temperature, humidity, and wind affect fire behavior.

**Typical Pattern (Clear Sky):**

| Time | Temperature | RH | Wind | Fire Behavior |
|------|-------------|-----|------|---------------|
| 0600 | Minimum | Maximum | Calm/light | Minimum activity |
| 1000 | Rising | Falling | Increasing | Activity building |
| 1400-1600 | Maximum | Minimum | Maximum | Peak activity |
| 1800 | Falling | Rising | Decreasing | Activity decreasing |
| 2200 | Low | High | Light/variable | Low activity |

**Night Fire Behavior:**
- Traditionally reduced due to higher humidity, lower temperature
- Climate change increasing nighttime fire activity
- Fires may maintain intensity through night during extreme events
- Nocturnal inversions can trap smoke, reducing visibility

---

## 4. Fuel Behaviors

### 4.1 Fuel Moisture Effects

**Description:** Water content of fuel is the single most important factor determining fire behavior.

**Moisture Thresholds:**
| Fuel Moisture | Fire Behavior |
|---------------|---------------|
| < 3% | Extreme: near-explosive ignition |
| 3-6% | Very High: rapid sustained spread |
| 6-12% | High: active fire spread |
| 12-25% | Moderate: fire spread possible |
| > 25% | Low: fire unlikely to spread |

**Fuel Moisture Timelag Classes:**

| Class | Diameter | Response Time | Example |
|-------|----------|---------------|---------|
| 1-hour | < 6mm | 1 hour | Fine twigs, grass |
| 10-hour | 6-25mm | 10 hours | Small branches |
| 100-hour | 25-75mm | 100 hours (~4 days) | Medium branches |
| 1000-hour | 75-200mm | 1000 hours (~42 days) | Large logs |

**Live Fuel Moisture:**
- Seasonal variation based on plant phenology
- Drought stress reduces live fuel moisture
- Critical threshold varies by species (typically 80-120%)

**Primary Model:** Nelson (2000) fuel moisture model
```
EMC = equilibrium moisture content
FMC(t+Δt) = EMC + (FMC(t) - EMC) × e^(-Δt/τ)

Where:
τ = timelag constant (hours)
EMC = function of temperature and RH
```

---

### 4.2 Fuel Types

**Description:** Different vegetation types exhibit distinct fire behavior characteristics.

**Major Fuel Categories:**

| Type | Characteristics | Spread Rate | Intensity |
|------|-----------------|-------------|-----------|
| **Grass** | Fine, continuous, rapid drying | Very fast (up to 25 km/h) | Low-moderate |
| **Shrub** | Mixed sizes, variable continuity | Moderate (3-10 km/h) | Moderate-high |
| **Forest litter** | Compact, shaded, slower drying | Slow-moderate (0.5-3 km/h) | Variable |
| **Forest crown** | Elevated, wind-exposed | Fast when active (5-20 km/h) | Very high |

**Fuel Bed Properties:**

| Property | Definition | Effect on Fire |
|----------|------------|----------------|
| Fuel load | Mass per area (kg/m²) | Energy available |
| Fuel depth | Height of fuel bed (m) | Reaction zone size |
| Bulk density | Mass per volume (kg/m³) | Heat transfer efficiency |
| Packing ratio | Fraction of bed volume occupied by fuel | Optimal ~0.007 for spread |
| Surface area-to-volume ratio | Surface area per volume (m⁻¹) | Drying rate, ignition ease |

---

### 4.3 Bark Shedding and Spotting

**Description:** Bark type and shedding behavior determines spotting potential.

**Eucalyptus Bark Classifications:**

| Bark Type | Species Examples | Spotting Hazard | Description |
|-----------|------------------|-----------------|-------------|
| **Ribbon/Candlebark** | E. viminalis, E. rubida | Extreme | Long strips that curl and burn for 30+ min |
| **Stringybark** | E. obliqua, E. macrorhyncha | Very High | Fibrous bark easily ignited, good lofting |
| **Box bark** | E. melliodora, E. polyanthemos | High | Short-fibered, burns quickly |
| **Ironbark** | E. sideroxylon, E. crebra | Moderate | Hard bark, fragments less easily |
| **Smooth bark** | E. regnans, E. globulus | Low-Moderate | Thin, shed bark at base |

**Bark Firebrand Characteristics:**
- Terminal velocity: 3-8 m/s depending on shape
- Burnout time: 2-40 minutes
- Lofting height: proportional to flame height × 12.2

---

### 4.4 Eucalyptus Oil Volatilization

**Description:** Volatile oils in eucalyptus leaves vaporize and ignite explosively under fire conditions.

**Chemistry:**
- Primary compounds: Cineole (eucalyptol), pinene, limonene
- Oil content: 2-5% of leaf dry weight
- Heat of combustion: ~43 MJ/kg (similar to gasoline)

**Critical Temperatures:**
| Process | Temperature |
|---------|-------------|
| Oil vaporization begins | ~170°C |
| Flash point | ~170-200°C |
| Autoignition | 230-260°C |
| Explosive vapor ignition | ~250°C |

**Effects:**
- Gas-phase combustion ahead of flame front
- "Explosive" crown fire transitions
- Blue flame appearance from volatile combustion
- Enhanced radiant and convective heat flux

**Australian Significance:**
- Eucalyptus forests exhibit distinctive "explosive" crown fire behavior
- Oil vapor clouds can ignite simultaneously over large areas
- Contributes to extreme fire intensity unique to Australian forests

---

### 4.5 Dead vs Live Fuel

**Description:** Dead and live fuels behave differently in fires.

**Dead Fuel:**
- Moisture controlled by weather
- Responds to humidity changes
- Primary carrier of surface fire
- Fine dead fuel < 6mm diameter most critical

**Live Fuel:**
- Moisture controlled by plant physiology
- Seasonal variation
- Higher ignition temperature when moist
- Can act as fire barrier or carrier depending on moisture

**Live Fuel Moisture Effects:**
```
Ignition threshold varies by species:
- Grass: 200-300% when green
- Eucalyptus: 80-150%
- Pine: 100-180%

Below these thresholds, live fuel actively participates in fire spread.
```

---

### 4.6 Fuel Continuity

**Description:** Gaps in fuel affect fire spread ability.

**Horizontal Continuity:**
- Continuous fuel → uninterrupted spread
- Gaps > 2× flame length may stop fire
- Roads, rivers, bare ground as firebreaks

**Vertical Continuity (Ladder Fuels):**
- Connects surface to crown fuels
- Shrubs, bark, low branches
- Critical for crown fire initiation

**Modified Van Wagner for Ladder Fuels:**
```
Effective CBH = CBH - (Ladder fuel height contribution)

Ladder fuel reduces effective crown base height,
lowering intensity threshold for crown ignition.
```

---

## 5. Combustion Phases

### 5.1 Preheating/Drying Phase

**Description:** Fuel ahead of fire front receives heat, driving off moisture before ignition.

**Processes:**
1. Radiant heating from flame front
2. Convective heating from hot gases
3. Conducted heat through fuel particles
4. Moisture evaporation (2260 kJ/kg latent heat)

**Temperature Progression:**
- Ambient → 100°C: Fuel heating
- 100°C plateau: Moisture evaporates (latent heat absorbed)
- 100°C → 200°C: Continued heating after drying
- 200°C → 300°C: Pyrolysis begins

**Key Parameter:** Heat required for ignition
```
Q_ig = (T_ig - T_ambient) × C_p + M × L_v

Where:
T_ig = ignition temperature (~300°C)
C_p = specific heat capacity (~1.5 kJ/kg·K)
M = moisture content (fraction)
L_v = latent heat of vaporization (2260 kJ/kg)
```

---

### 5.2 Ignition

**Description:** Transition from preheating to active combustion.

**Ignition Types:**
| Type | Description | Temperature |
|------|-------------|-------------|
| Piloted ignition | Flame or spark present | 250-300°C |
| Autoignition | No external ignition source | 400-500°C |
| Smoldering ignition | Low-temperature oxidation | 200-250°C |

**Factors Affecting Ignition:**
- Heat flux (kW/m²)
- Duration of exposure
- Fuel moisture
- Fuel size (surface area-to-volume ratio)
- Fuel chemistry

---

### 5.3 Flaming Combustion

**Description:** Gas-phase combustion with visible flames.

**Characteristics:**
- Temperature: 800-1200°C
- Duration: seconds to minutes per fuel element
- Products: CO₂, H₂O, CO, particulates
- Heat release: ~18,000 kJ/kg (dry wood)

**Heat Transfer from Flames:**
- **Radiation:** Stefan-Boltzmann law, dominant at distance
- **Convection:** Hot gas contact, dominant in flame zone
- **Conduction:** Through solid fuel, relatively minor

**Flame Geometry:**
- Height proportional to intensity^0.46 (Byram's flame length)
- Angle affected by wind and slope
- Residence time affects heat transfer to fuel

---

### 5.4 Smoldering Combustion

**Description:** Flameless oxidation of char or duff, low temperature, long duration.

**Primary Model:** Rein (2009)
```
Smoldering characteristics:
- Temperature: 400-600°C
- Spread rate: 1-10 cm/hour
- Oxygen limited (diffusion controlled)
- Can persist for days to months
```

**Conditions for Smoldering:**
- Fuel with high carbon content (peat, duff, heavy fuels)
- Sufficient oxygen but limited airflow
- Moisture content < 40% (varies by fuel)
- Initial ignition from flaming fire

**Effects:**
- Ground heating and root damage
- Delayed re-ignition potential
- Smoke production
- Fuel consumption continuation after flaming

---

### 5.5 Re-ignition from Smoldering

**Description:** Transition from smoldering back to flaming combustion.

**Triggers:**
- Wind increase exposing hot material to oxygen
- Fuel moisture decrease
- Fresh fuel contact with smoldering zone
- Mechanical disturbance

**Holdover Fires:**
- Fire persisting in smoldering state
- Can survive overnight, through rain, for days/weeks
- Lightning-ignited fires may smolder for weeks before flaming
- Peat fires can overwinter and re-emerge in spring ("zombie fires")

**Key Parameters:**
- Smoldering fuel temperature
- Available oxygen
- Fuel moisture of nearby fuels
- Wind exposure

---

## 6. Fire Intensity Effects

### 6.1 Flame Height

**Description:** Vertical extent of flames, indicator of fire intensity and energy release.

**Byram's Flame Height Relationship:**
```
L = 0.0775 × I_B^0.46

Where:
L = flame length (m)
I_B = fireline intensity (kW/m)

Inverse:
I_B = 259.833 × L^2.174
```

**Flame Height Interpretation:**
| Flame Length | Intensity | Suppression Capability |
|--------------|-----------|----------------------|
| < 1.2m | < 350 kW/m | Hand tools effective |
| 1.2-2.4m | 350-1700 kW/m | Machine line, tankers |
| 2.4-3.4m | 1700-3500 kW/m | Air support needed |
| > 3.4m | > 3500 kW/m | Control unlikely |

---

### 6.2 Fireline Intensity (Byram's Equation)

**Description:** Rate of energy release per unit length of fire front.

**Byram's Equation:**
```
I_B = H × w × R

Where:
I_B = fireline intensity (kW/m)
H = heat of combustion (kJ/kg) - typically 18,000-20,000
w = fuel consumed (kg/m²)
R = rate of spread (m/s)
```

**Intensity Classes:**
| Class | Intensity (kW/m) | Description |
|-------|------------------|-------------|
| Very Low | < 100 | Low flames, easily suppressed |
| Low | 100-500 | Moderate flames |
| Moderate | 500-2000 | Active fire behavior |
| High | 2000-4000 | Difficult suppression |
| Very High | 4000-10000 | Crown fire possible |
| Extreme | > 10000 | Crown fire, spotting, pyroCb |

---

### 6.3 Heat Flux (Radiant and Convective)

**Description:** Rate of heat transfer to fuel and surroundings.

**Radiant Heat Flux:**
```
Stefan-Boltzmann Law:
q_r = ε × σ × (T_source^4 - T_target^4)

Where:
q_r = radiant heat flux (W/m²)
ε = emissivity (0.7-0.9 for flames)
σ = Stefan-Boltzmann constant (5.67 × 10⁻⁸ W/m²K⁴)
T = absolute temperature (K)
```

**View Factor Effects:**
- Distance from flame reduces flux by 1/r²
- Flame geometry affects radiation pattern
- Smoke attenuates radiation slightly

**Convective Heat Flux:**
```
q_c = h × (T_gas - T_surface)

Where:
h = convective heat transfer coefficient (10-100 W/m²K)
Coefficient depends on wind speed and turbulence
```

**Relative Importance:**
- At distance: Radiation dominates
- In flame contact: Convection significant
- Preheating: Radiation primary mechanism

---

### 6.4 Fire Danger Indices

**Description:** Integrated measures of fire weather and danger.

**McArthur Forest Fire Danger Index (FFDI):**
```
FFDI = 2 × exp(-0.45 + 0.987 × ln(D) - 0.0345 × H + 0.0338 × T + 0.0234 × V)

Where:
D = Drought Factor (0-10)
H = Relative Humidity (%)
T = Temperature (°C)
V = Wind speed (km/h at 10m)
```

**FFDI Rating Scale:**
| FFDI | Rating | Description |
|------|--------|-------------|
| 0-11 | Low-Moderate | Fires controllable |
| 12-24 | High | Fires may be difficult to control |
| 25-49 | Very High | Fires very difficult to control |
| 50-74 | Severe | Uncontrollable fires |
| 75-99 | Extreme | Widespread uncontrollable fires |
| 100+ | Catastrophic | Black Saturday conditions |

**Canadian Fire Weather Index (FWI):**
- Uses similar weather inputs
- Includes fuel moisture codes (FFMC, DMC, DC)
- Build-up Index (BUI) and Initial Spread Index (ISI)
- Combines into FWI for fire danger rating

---

## 7. Extreme Fire Behaviors

### 7.1 Blow-Up Fires

**Description:** Sudden transition from normal to extreme fire behavior.

**Definition:** Sudden increase in fire intensity or rate of spread sufficient to preclude direct control.

**Characteristics:**
- Rapid spread rate increase (often 10x or more)
- Major convection column development
- Prolific spotting
- Often associated with plume attachment

**Contributing Factors:**
1. Low-level atmospheric instability
2. Steep terrain
3. High fuel loads
4. Low fuel moisture
5. Wind alignment with terrain

**Conditions Checklist:**
- Haines Index 5-6
- Temperature > 30°C
- RH < 25%
- FFDI > 50
- Slope > 30%
- Continuous heavy fuels

---

### 7.2 Firestorms

**Description:** Large area fire creating its own weather system with hurricane-force indrafts.

**Characteristics:**
- Fire area typically > 1000 hectares
- Intense convection creating storm-force winds
- Self-sustaining once established
- Indraft winds from all directions toward fire center
- pyroCb development common

**Historical Examples:**
- Hamburg (1943): Urban firestorm, 35,000+ deaths
- Black Saturday (2009): Multiple firestorms
- 2019-20 Australian bushfire season: Multiple firestorms

---

### 7.3 Mass Spotting (Ember Storms)

**Description:** Prolific production and transport of firebrands creating widespread ignitions.

**Causes:**
- Convective column collapse
- pyroCb downdrafts
- Wind shift during intense fire
- Junction zone formation

**Characteristics:**
- Hundreds to thousands of spot fires
- Spotting distances: 1-30+ km
- Overwhelms suppression capability
- Creates merged fire perimeter rapidly

---

### 7.4 Junction Zone Fires (Converging Fires)

**Description:** Interaction zone where two fire fronts merge, creating accelerated fire behavior.

**Scientific Basis:**
- Combined radiant heat flux at junction point
- Air entrained from both sides
- Indraft convergence creates enhanced updraft
- Rate of spread at junction > sum of individual fires

**Key Parameters:**
- Angle between fire fronts (θ)
- Individual fire spread rates
- Fire front lengths

**Acceleration Factor:**
```
Junction fires with acute angles (< 45°) show maximum acceleration
Peak ROS can be 2-5× individual fire ROS
Effect diminishes as fires become parallel (θ → 0°)
```

**Safety Implications:**
- Junction zones are extremely hazardous
- Fire behavior changes rapidly and unpredictably
- Major factor in firefighter fatalities

---

### 7.5 Night vs Day Behavior Differences

**Description:** Fire behavior varies with diurnal cycle.

**Day vs Night Comparison:**

| Factor | Daytime | Nighttime |
|--------|---------|-----------|
| Temperature | Higher | Lower |
| Relative Humidity | Lower | Higher |
| Fuel Moisture | Lower | Higher (if dew) |
| Wind Speed | Higher, gusty | Lower, steadier |
| Stability | Unstable | Stable (inversion) |
| Fire Activity | Higher | Lower |
| Visibility | Better | Reduced by smoke |

**Nocturnal Fire Behavior:**
- Traditional doctrine: fires "lay down" at night
- Reality: extreme conditions can maintain intensity overnight
- Black Saturday: fires burned intensely through night
- Climate change increasing nighttime fire activity

---

### 7.6 Fire Flanks vs Head Fire

**Description:** Different parts of fire perimeter exhibit distinct behavior.

**Fire Perimeter Anatomy:**

```
              ↑ Wind direction
              
        Head Fire (fastest spread)
        ↗        ↑        ↖
       /                    \
Left  /                      \  Right
Flank |                      |  Flank
      |                      |
       \                    /
        ↘        ↓        ↙
         Backing/Rear Fire
         (slowest spread)
```

**Relative Spread Rates:**
| Fire Part | Typical ROS Ratio | Intensity |
|-----------|-------------------|-----------|
| Head | 1.0 (reference) | Highest |
| Flanks | 0.1-0.3 | Moderate |
| Backing | 0.05-0.1 | Lowest |

**Elliptical Fire Shape:**
- Length-to-breadth ratio increases with wind speed
- Light wind: nearly circular (L:B ≈ 1.5)
- Strong wind: highly elongated (L:B > 4)

---

### 7.7 Plume-Dominated vs Wind-Driven Fires

**Description:** Two fundamental fire behavior regimes.

**Wind-Driven Fire:**
- Spread controlled by ambient wind
- Predictable direction
- Tilted plume
- Flames angled toward unburned fuel
- More common regime

**Plume-Dominated Fire:**
- Spread controlled by fire-generated convection
- Strong vertical plume development
- May spread in any direction
- Erratic behavior
- Associated with blow-up potential

**Transition Between Regimes:**
- Depends on fire intensity and wind speed
- Low wind + high intensity → plume-dominated
- High wind + any intensity → wind-driven
- Transition is dangerous moment

---

## 8. Secondary Effects

### 8.1 Oxygen Depletion

**Description:** Combustion consumes oxygen, potentially creating localized oxygen-deficient zones.

**Chemistry:**
```
Combustion equation (simplified):
CH₂O (fuel) + O₂ → CO₂ + H₂O

Atmospheric O₂ ≈ 21%
Combustion impaired below ~16%
Life-threatening below ~12%
```

**Effects:**
- Incomplete combustion → more CO
- Localized effects near intense fire
- Generally not significant in open wildfires
- May be factor in entrapment situations

---

### 8.2 Smoke Effects

**Description:** Smoke affects visibility, radiation, and health.

**Visibility Reduction:**
- Dense smoke can reduce visibility to < 10m
- Affects firefighter safety and operations
- Impacts aviation and ground transportation

**Radiation Attenuation:**
- Smoke particles scatter and absorb radiation
- Reduces solar input to surface
- Slightly reduces radiant heating from flames at distance
- "Smoke column shading" effect

**Smoke Composition:**
- Particulate matter (PM2.5, PM10)
- Carbon monoxide (CO)
- Carbon dioxide (CO₂)
- Volatile organic compounds
- Polycyclic aromatic hydrocarbons

---

### 8.3 Tree Fall from Burnout

**Description:** Trees weakened by fire fall unpredictably.

**Causes:**
- Root damage from ground fire
- Trunk burnthrough
- Crown weight redistribution
- Delayed structural failure

**Hazard Duration:**
- Peak hazard: 24-72 hours post-fire
- Continues for days to weeks
- "Widow-makers" major post-fire hazard

---

### 8.4 Post-Fire Smoldering and Re-ignition

**Description:** Fire persistence after main fire passes.

**Smoldering Locations:**
- Heavy fuels (logs, stumps)
- Root systems
- Peat and organic soils
- Accumulated duff

**Re-ignition Risks:**
- Wind increase
- Fuel moisture decrease
- Mechanical disturbance
- May occur days to weeks after initial fire

**Monitoring Required:**
- Infrared detection of hot spots
- Multi-day patrol after fire passage
- Mop-up operations for containment lines

---

## 9. Fire Geometry and Spread Patterns

### 9.1 Elliptical Fire Growth Model

**Description:** Fires spread in elliptical shape under steady wind conditions.

**Van Wagner (1969) Model:**
```
Fire perimeter approximated as double ellipse:
- Forward (head) ellipse: faster spread
- Rear (backing) ellipse: slower spread

Length-to-breadth ratio:
L/B = 1 + 0.25 × V

Where V = wind speed (km/h)

Area = π × a × b (ellipse area formula)

Where:
a = semi-major axis (head fire distance)
b = semi-minor axis (flank fire distance)
```

**Time Evolution:**
- Fire area increases approximately quadratically with time
- Perimeter increases linearly with time (initially)
- Complexity increases as fire interacts with terrain/fuel

---

### 9.2 Point Source Ignition

**Description:** Fire spreading from single ignition point.

**Initial Development:**
- Spreads radially until one direction dominates
- Wind and slope cause asymmetric development
- Transition to elliptical shape

**Time to Establish Head Fire:**
- Depends on fuel type and weather
- Typically 5-30 minutes for distinct head
- Critical for initial attack window

---

### 9.3 Line Ignition

**Description:** Fire ignited along a line (natural or prescribed fire technique).

**Behavior:**
- Fire spreads perpendicular to ignition line
- Used in prescribed burning for control
- Creates uniform fire front

**Head Fire from Line:**
- Develops across entire line simultaneously
- No single point concentration
- Different tactical implications

---

## Appendix A: Key Formulas Summary

### Heat Transfer
```
Stefan-Boltzmann: q = εσ(T₁⁴ - T₂⁴)
Convective: q = h(T_gas - T_surface)
```

### Fire Spread
```
Rothermel: R = (I_R × ξ × (1 + φ_w + φ_s)) / (ρ_b × ε × Q_ig)
```

### Crown Fire Initiation (Van Wagner)
```
I₀ = (0.010 × CBH × (460 + 25.9 × FMC))^(3/2)
```

### Fireline Intensity
```
Byram: I_B = H × w × R
```

### Flame Length
```
L = 0.0775 × I^0.46
```

### FFDI
```
FFDI = 2 × exp(-0.45 + 0.987×ln(D) - 0.0345×H + 0.0338×T + 0.0234×V)
```

### Spotting (Albini)
```
Maximum loft height: z_max = 12.2 × H_F
```

### Fuel Moisture Response
```
FMC(t+Δt) = EMC + (FMC(t) - EMC) × e^(-Δt/τ)
```

---

## Appendix B: Australian-Specific Considerations

1. **Eucalyptus Volatile Oils**
   - Vaporization: 170°C
   - Autoignition: 230-260°C
   - Heat content: 43 MJ/kg

2. **Ribbon Bark Spotting**
   - Maximum: 30+ km
   - Typical severe: 10-20 km
   - Burns for 30+ minutes

3. **FFDI Scale**
   - Catastrophic: 100+
   - Used in all states/territories
   - Basis for fire danger ratings

4. **Fire Season**
   - Northern Australia: May-November (dry season)
   - Southern Australia: October-March (summer)
   - Overlap periods most dangerous

5. **Key Historical Fires**
   - Black Saturday 2009: FFDI 180+
   - Ash Wednesday 1983: Cold front disasters
   - Black Summer 2019-20: 18 pyroCb events

---

## Appendix C: Simulation Completeness Checklist

> **Last Audit:** 29 December 2025  
> **Auditor:** GitHub Copilot comprehensive analysis

### Must-Have Behaviors

| Behavior | Status | Implementation | Notes |
|----------|--------|----------------|-------|
| Surface fire spread (Rothermel) | ✅ IMPLEMENTED | `physics/rothermel.rs` | Full Rothermel (1972) model |
| Slope effects (φ_s factor) | ⚠️ PARTIAL | Legacy: `terrain_physics.rs`; Field solver: MISSING | Phase 0 in FIRE_PHYSICS_ENHANCEMENTS.md |
| Wind effects (φ_w factor) | ✅ IMPLEMENTED | `solver/level_set.rs`, `element_heat_transfer.rs` | 26× downwind asymmetry |
| Fuel moisture effects | ✅ IMPLEMENTED | `physics/fuel_moisture.rs` | Nelson (2000) timelag model |
| Heat transfer (radiant + convective) | ✅ IMPLEMENTED | `solver/heat_transfer.rs`, `element_heat_transfer.rs` | Stefan-Boltzmann + convection |
| Ignition thresholds | ✅ IMPLEMENTED | `solver/combustion.rs` | T_ign ~300°C |
| Fireline intensity (Byram) | ✅ IMPLEMENTED | `physics/byram_intensity.rs` | I = H × w × R |
| Flame height calculation | ✅ IMPLEMENTED | `physics/byram_intensity.rs` | L = 0.0775 × I^0.46 |
| Fire danger index (FFDI) | ✅ IMPLEMENTED | `core_types/weather.rs` | McArthur Mk5 formula |

### Should-Have Behaviors

| Behavior | Status | Implementation | Notes |
|----------|--------|----------------|-------|
| Crown fire initiation (Van Wagner) | ✅ IMPLEMENTED | `physics/crown_fire.rs` | I₀ formula complete |
| Crown fire spread | ✅ IMPLEMENTED | `physics/crown_fire.rs` | Active/passive types |
| Spotting/ember transport (Albini) | ✅ IMPLEMENTED | `physics/albini_spotting.rs`, `core_types/ember.rs` | 25km validated |
| Fuel moisture timelag response | ✅ IMPLEMENTED | `physics/fuel_moisture.rs` | 1h, 10h, 100h classes |
| Diurnal weather variation | ✅ IMPLEMENTED | `core_types/weather.rs` | Temperature/humidity cycles |
| Terrain aspect effects | ⚠️ PARTIAL | Legacy: `terrain_physics.rs`; Field solver: MISSING | Phase 0 in task file |
| Smoldering combustion | ✅ IMPLEMENTED | `physics/smoldering.rs` | Rein (2009) model |
| Elliptical fire shape | ✅ IMPLEMENTED | `solver/level_set.rs`, `element_heat_transfer.rs` | Anderson (1983) ellipse |

### Advanced Behaviors

| Behavior | Status | Implementation | Notes |
|----------|--------|----------------|-------|
| Pyroconvection effects | ⚠️ CODE EXISTS | `weather/pyrocumulus.rs` | Not wired to sim; Phase 4 in task file |
| Fire whirl detection | ⚠️ CODE EXISTS | `weather/pyrocumulus.rs` | `check_fire_tornado_risk()` exists |
| VLS (vorticity-driven lateral spread) | ❌ NOT IMPLEMENTED | — | Future enhancement needed |
| Junction zone acceleration | ❌ NOT IMPLEMENTED | — | Two fires merging; future work |
| Eucalyptus oil volatilization | ⚠️ PARTIAL | Fuel heat content (43 MJ/kg); volatilization physics not explicit | Data exists, physics implicit |
| Bark-type spotting differences | ✅ IMPLEMENTED | `core_types/ember.rs`, `physics/canopy_layers.rs` | Stringybark vs smooth bark |
| Plume-dominated vs wind-driven | ❌ NOT IMPLEMENTED | — | Regime detection needed |
| pyroCb-induced downdrafts | ⚠️ CODE EXISTS | `weather/pyrocumulus.rs` | Mentioned but not active |
| Valley channeling/chimney effect | ❌ NOT IMPLEMENTED | — | Terrain + wind coupling needed |
| Mass spotting (ember storms) | ⚠️ PARTIAL | Multi-ember system exists | No "storm" detection/enhancement |
| Night fire behavior | ✅ IMPLEMENTED | `core_types/weather.rs` | Diurnal cycles affect fire |
| Wind shift effects | ⚠️ PARTIAL | `grid/wind_field.rs` | Wind changes; flank→head transition implicit |
| Atmospheric stability indices | ✅ IMPLEMENTED | `weather/atmosphere.rs` | Haines, Lifted Index, K-Index |
| Ladder fuels (surface→crown) | ✅ IMPLEMENTED | `physics/canopy_layers.rs` | Stringybark factor 0.9 |
| Oxygen depletion | ⚠️ PARTIAL | `suppression/agent.rs` | For suppression; not ambient |
| Post-fire re-ignition | ⚠️ PARTIAL | `physics/smoldering.rs` | Smoldering exists; explicit re-ignition not modeled |
| Terrain ridge wind blocking | ✅ IMPLEMENTED | `grid/wind_field.rs` | `apply_terrain_blocking()` |

### Summary Statistics

| Category | Implemented | Partial | Missing |
|----------|-------------|---------|---------|
| **Must-Have** | 7/9 (78%) | 2/9 (22%) | 0/9 (0%) |
| **Should-Have** | 6/8 (75%) | 2/8 (25%) | 0/8 (0%) |
| **Advanced** | 5/18 (28%) | 8/18 (44%) | 5/18 (28%) |
| **TOTAL** | 18/35 (51%) | 12/35 (34%) | 5/35 (14%) |

### Priority Fixes Required

1. **🔥 CRITICAL: Terrain Slope in Field Solver** (Phase 0)
   - Fire spreads same rate uphill/downhill
   - Should be 2× faster per 10° uphill (McArthur 1967)

2. **HIGH: Wire Pyroconvection to Simulation** (Phase 4)
   - Code exists but not connected
   - Critical for extreme fire behavior

3. **MEDIUM: Junction Zone Physics**
   - Two fires merging creates 2-5× acceleration
   - Important for fire safety

4. **MEDIUM: VLS (Vorticity-Driven Lateral Spread)**
   - Identified in 2003 Canberra fires
   - Steep slopes + wind = lateral fire runs

5. **LOW: Valley Channeling Effect**
   - Wind accelerates in valleys
   - Fire spreads faster in confined terrain

---

## References

1. Rothermel, R.C. (1972). A mathematical model for predicting fire spread in wildland fuels. USDA Forest Service Research Paper INT-115.

2. Van Wagner, C.E. (1977). Conditions for the start and spread of crown fire. Canadian Journal of Forest Research 7:23-34.

3. Albini, F.A. (1979). Spot fire distance from burning trees - a predictive model. USDA Forest Service General Technical Report INT-56.

4. Byram, G.M. (1959). Combustion of forest fuels. In: Forest Fire: Control and Use.

5. Nelson, R.M. (2000). Prediction of diurnal change in 10-h fuel stick moisture content. Canadian Journal of Forest Research 30:1071-1087.

6. Rein, G. (2009). Smouldering combustion phenomena in science and technology. International Review of Chemical Engineering 1:3-18.

7. McArthur, A.G. (1967). Fire behaviour in eucalypt forests. Forestry and Timber Bureau Leaflet 107.

8. Cruz, M.G. & Alexander, M.E. (2014). The 10% wind speed rule of thumb for estimating a wildfire's forward rate of spread in forests and shrublands.

9. Sharples, J.J. et al. (2012). Wind-terrain effects on the propagation of wildfires in rugged terrain: fire channelling. International Journal of Wildland Fire 21:282-296.

10. USDA Forest Service (2013). Synthesis of Knowledge of Extreme Fire Behavior: Volume I for Fire Managers. PNW-GTR-854.
