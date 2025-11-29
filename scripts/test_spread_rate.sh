#!/bin/bash
# Test fire spread rate after removing heat_boost multiplier
# Expected: Perth Metro (FFDI ~11) should spread at 1-10 ha/hr, not 29,880 ha/hr

echo "╔═══════════════════════════════════════════════════════════╗"
echo "║         Fire Spread Rate Validation Test                  ║"
echo "╚═══════════════════════════════════════════════════════════╝"
echo ""
echo "Testing Perth Metro conditions (FFDI ~11, Moderate danger)"
echo "Expected spread rate: 1-10 hectares/hour for moderate conditions"
echo ""
echo "Commands to run:"
echo "  1. Type: 1000  (terrain width)"
echo "  2. Press Enter (terrain width)"
echo "  3. Type: 1000  (terrain height)"
echo "  5. Press Enter (terrain height)"
echo "  6. Type: p perth"
echo "  7. Press Enter (perth is the weather preset)"
echo "  8. Type: i 7"
echo "  9. Press Enter (i = ignite, 7 is the element id)"
echo "  10. Type: s 100"
echo "  11. Press Enter (s = step, 100 is the amount of steps)"
echo ""
echo "Watch the burning element count after 100 steps (10 seconds)"
echo "Calculate: (burning_count / 57956) × 83 hectares = area burned"
echo "Then: area × 360 = hectares/hour"
echo ""
echo "Target result: Should be 10-100 ha/hr (MUCH slower than before)"
echo "Previous result: 29,880 ha/hr (WAY TOO FAST)"
echo ""
echo "Press Enter to start demo..."
read

cd "/mnt/Main_Data/Game Projects/BFS (Bushfire Simulator)"
./target/release/demo-interactive
