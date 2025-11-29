#!/usr/bin/env python3
"""
Fire Simulation Profile Analyzer

Analyzes samply profiling data to identify performance hotspots.

Usage:
    python3 scripts/analyze_profile.py profile.json profile.syms.json
    python3 scripts/analyze_profile.py profile.json.gz profile.syms.json

This script is called after profiling with:
    samply record --save-only target/release/demo-interactive
    
Output shows top hotspots and key fire simulation functions.
Supports both plain JSON and gzip-compressed files.
"""

import gzip
import json
import sys
from collections import defaultdict
from pathlib import Path


def load_profile_data(profile_path, symbols_path):
    """Load profile and symbol data. Automatically handles gzip compression."""
    # Load profile (may be gzipped)
    if str(profile_path).endswith('.gz'):
        with gzip.open(profile_path, 'rt', encoding='utf-8') as f:
            profile = json.load(f)
    else:
        with open(profile_path, 'r') as f:
            profile = json.load(f)
    
    # Load symbols (may be gzipped)
    if str(symbols_path).endswith('.gz'):
        with gzip.open(symbols_path, 'rt', encoding='utf-8') as f:
            symbols = json.load(f)
    else:
        with open(symbols_path, 'r') as f:
            symbols = json.load(f)
    
    return profile, symbols


def build_address_map(symbols):
    """Build mapping from address to symbol name for demo-interactive."""
    demo_lib = None
    for lib in symbols['data']:
        if 'demo-interactive' in lib.get('debug_name', ''):
            demo_lib = lib
            break
    
    if not demo_lib:
        print("ERROR: Could not find demo-interactive library in symbols")
        sys.exit(1)
    
    addr_to_sym = {}
    for sym_entry in demo_lib['symbol_table']:
        rva = sym_entry['rva']
        size = sym_entry['size']
        symbol_idx = sym_entry['symbol']
        symbol_name = symbols['string_table'][symbol_idx]
        
        # Map all addresses in this range
        for offset in range(size):
            addr_to_sym[rva + offset] = symbol_name
    
    return addr_to_sym


def find_main_thread(profile):
    """Find the main thread with actual samples."""
    for thread in profile['threads']:
        if 'demo-interactive' in thread.get('processName', '') or thread.get('isMainThread'):
            if thread['samples']['length'] > 100:
                return thread
    
    # Fallback: find thread with most samples
    return max(profile['threads'], key=lambda t: t['samples']['length'])


def analyze_samples(thread, addr_to_sym):
    """Count samples per function by walking stacks."""
    func_samples = defaultdict(int)
    stack_indices = thread['samples']['stack']
    
    for stack_idx in stack_indices:
        if stack_idx is None or stack_idx < 0:
            continue
        
        # Walk up the stack (limit depth to avoid infinite loops)
        current = stack_idx
        visited = set()
        
        for depth in range(50):
            if current is None or current < 0:
                break
            
            if current in visited:
                break
            visited.add(current)
            
            frame_idx = thread['stackTable']['frame'][current]
            addr = thread['frameTable']['address'][frame_idx]
            
            # Look up symbol name
            func_name = addr_to_sym.get(addr, f'0x{addr:x}')
            func_samples[func_name] += 1
            
            # Get parent stack frame
            current = thread['stackTable']['prefix'][current]
    
    return func_samples, len(stack_indices)


def print_analysis(func_samples, total_samples):
    """Print hotspot analysis."""
    print(f'Total samples: {total_samples}\n')
    
    # Key fire simulation functions to track
    key_functions = [
        ('fire_sim_core::simulation::FireSimulation::update', 'UPDATE'),
        ('fire_sim_core::grid::simulation_grid::SimulationGrid::mark_active_cells', 'MARK_ACTIVE'),
        ('fire_sim_core::grid::simulation_grid::SimulationGrid::update_diffusion', 'DIFFUSION'),
        ('fire_sim_core::core_types::spatial::SpatialIndex::query_radius', 'QUERY_RADIUS'),
        ('fire_sim_core::physics::element_heat_transfer::calculate_total_heat_transfer', 'HEAT_XFER'),
        ('core::iter::range::<impl core::iter::traits::iterator::Iterator for core::ops::range::Range<A>>::next', 'RANGE_NEXT'),
        ('<core::ops::range::Range<T> as core::iter::range::RangeIteratorImpl>::spec_next', 'SPEC_NEXT'),
        ('hashbrown::raw::RawTableInner::find_or_find_insert_slot_inner', 'HASHMAP_FIND'),
        ('rayon::iter::extend::<impl rayon::iter::ParallelExtend<T> for alloc::vec::Vec<T>>::par_extend', 'PAR_EXTEND'),
        ('<core::slice::iter::Iter<T> as core::iter::traits::iterator::Iterator>::next', 'SLICE_ITER'),
        ('alloc::vec::Vec<T,A>::push', 'VEC_PUSH'),
    ]
    
    print('=' * 80)
    print('KEY FIRE SIMULATION FUNCTIONS')
    print('=' * 80)
    for func_name, label in key_functions:
        count = func_samples.get(func_name, 0)
        pct = (count / total_samples * 100) if total_samples > 0 else 0
        if count > 0:
            print(f'{pct:5.1f}%  {count:6d}  {label:20s} {func_name}')
    
    print('\n' + '=' * 80)
    print('TOP 40 HOTSPOTS (fire_sim_core, hashbrown, rayon, core::iter)')
    print('=' * 80)
    
    # Filter and sort hotspots
    sorted_funcs = sorted(func_samples.items(), key=lambda x: -x[1])
    
    keywords = ['fire_sim_core', 'hashbrown', 'rayon', 'core::iter', 'alloc::vec', 'core::slice']
    
    count_shown = 0
    for func_name, count in sorted_funcs:
        if count_shown >= 40:
            break
        
        if any(kw in func_name for kw in keywords):
            pct = (count / total_samples * 100)
            print(f'{pct:5.1f}%  {count:6d}  {func_name}')
            count_shown += 1
    
    print('\n' + '=' * 80)
    print('PERFORMANCE SUMMARY')
    print('=' * 80)
    
    # Calculate key metrics
    update_pct = func_samples.get('fire_sim_core::simulation::FireSimulation::update', 0) / total_samples * 100
    mark_pct = func_samples.get('fire_sim_core::grid::simulation_grid::SimulationGrid::mark_active_cells', 0) / total_samples * 100
    diff_pct = func_samples.get('fire_sim_core::grid::simulation_grid::SimulationGrid::update_diffusion', 0) / total_samples * 100
    query_pct = func_samples.get('fire_sim_core::core_types::spatial::SpatialIndex::query_radius', 0) / total_samples * 100
    par_extend_pct = func_samples.get('rayon::iter::extend::<impl rayon::iter::ParallelExtend<T> for alloc::vec::Vec<T>>::par_extend', 0) / total_samples * 100
    
    print(f'Update:          {update_pct:5.1f}%  (Main simulation loop)')
    print(f'Query Radius:    {query_pct:5.1f}%  (Spatial neighbor searches)')
    print(f'Diffusion:       {diff_pct:5.1f}%  (Grid atmospheric updates)')
    print(f'Mark Active:     {mark_pct:5.1f}%  (Grid cell activation)')
    print(f'Par Extend:      {par_extend_pct:5.1f}%  (Rayon parallel overhead)')
    print()
    print(f'Top 3 bottlenecks: ', end='')
    
    bottlenecks = [
        ('Query Radius', query_pct),
        ('Diffusion', diff_pct),
        ('Mark Active', mark_pct),
        ('Par Extend', par_extend_pct),
    ]
    bottlenecks.sort(key=lambda x: -x[1])
    
    for i, (name, pct) in enumerate(bottlenecks[:3]):
        if i > 0:
            print(', ', end='')
        print(f'{name} ({pct:.1f}%)', end='')
    print()


def main():
    if len(sys.argv) != 3:
        print(__doc__)
        print(f"\nUsage: {sys.argv[0]} <profile.json[.gz]> <profile.syms.json[.gz]>")
        sys.exit(1)
    
    profile_path = Path(sys.argv[1])
    symbols_path = Path(sys.argv[2])
    
    if not profile_path.exists():
        print(f"ERROR: Profile file not found: {profile_path}")
        sys.exit(1)
    
    if not symbols_path.exists():
        print(f"ERROR: Symbols file not found: {symbols_path}")
        sys.exit(1)
    
    print("Loading profile data...")
    profile, symbols = load_profile_data(profile_path, symbols_path)
    
    print("Building address map...")
    addr_to_sym = build_address_map(symbols)
    print(f"  Loaded {len(addr_to_sym)} address mappings")
    
    print("Finding main thread...")
    main_thread = find_main_thread(profile)
    print(f"  Thread: {main_thread['name']}, Samples: {main_thread['samples']['length']}")
    
    print("\nAnalyzing samples...")
    func_samples, total_samples = analyze_samples(main_thread, addr_to_sym)
    
    print("\n")
    print_analysis(func_samples, total_samples)


if __name__ == '__main__':
    main()
