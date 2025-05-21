import os
import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import seaborn as sns
import argparse
from glob import glob

name_translator = {
        "faaaq_rust_optimised" : "FAAAQueue Optimised",
        "faaaq_rust_unoptimised" : "FAAAQueue Unoptimised",
        "faaa_queue_cpp" : "C++ FAAAQueue",
        "lprq_rust_correct" : "Rust LPRQ Optimised",
        "lcrq_rust_correct" : "Rust LCRQ Optimised",
        "lcrq_cpp" : "C++ LCRQ",
        "lprq_rust_unoptimised" : "Rust LPRQ Unoptimised",
        "lcrq_rust_unoptimised" : "Rust LCRQ Unoptimised",
        "lprq_cpp" : "C++ LPRQ",
        "lcrq_rust" : "Rust LCRQ",
        "lprq_rust" : "Rust LPRQ",
        "moodycamel_cpp" : "moodycamel (C++)",
        "seg_queue" : "SegQueue",
        "array_queue" : "ArrayQueue",
        "atomic_queue" : "atomic-queue",
        "basic_queue" : "BasicQueue",
        "bounded_ringbuffer" : "Bounded Ringbuffer",
        "bounded_concurrent_queue" : "concurrent_queue::bounded",
        "unbounded_concurrent_queue" : "concurrent_queue::unbounded",
        "lf_queue" : "lf-queue",
        "lockfree_queue" : "lockfree::Queue",
        "lockfree_stack" : "lockfree::Stack",
        "scc2_queue" : "scc2::Queue",
        "scc2_stack" : "scc2::Stack",
        "scc_queue" : "scc::Queue",
        "scc_stack" : "scc::Stack",
        "boost_cpp" : "boost (C++)",
        "faaa_queue_rust" : "Rust FAAAQueue",
        "tz_queue_hp" : "TsigasZhang (HP)",
        "bbq" : "bbq-rs",
        "ms_queue" : "MSQueue",
        }

def load_benchmark_file(file_path):
    """
    Load a benchmark file and return as DataFrame
    """
    df = pd.read_csv(file_path)
    return df

def find_peak_memory_in_file(file_path):
    """
    Find the peak memory allocation in a single file
    """
    try:
        df = load_benchmark_file(file_path)
        if 'Memory Allocated' in df.columns:
            peak_memory = df['Memory Allocated'].max()
            return peak_memory
        else:
            print(f"Warning: 'Memory Allocated' column not found in {file_path}")
            return None
    except Exception as e:
        print(f"Error processing {file_path}: {e}")
        return None

def process_queue_folder(folder_path, queue_type=None):
    """
    Process all mem* files in a folder and return average peak memory
    """
    # Get all mem* files in the folder
    file_paths = glob(os.path.join(folder_path, "mem*"))
    
    if not file_paths:
        print(f"No memory files found in {folder_path}")
        return None
    
    print(f"Found {len(file_paths)} memory files for queue type {queue_type}")
    
    # Process each file to find peak memory
    peak_memories = []
    for file_path in file_paths:
        peak_memory = find_peak_memory_in_file(file_path)
        if peak_memory is not None:
            peak_memories.append(peak_memory)
    
    # Calculate average peak memory across all files
    if peak_memories:
        avg_peak_memory = sum(peak_memories) / len(peak_memories)
        print(f"Mean peak memory for {queue_type}: {int(avg_peak_memory):,} bytes from {len(peak_memories)} files")
        
        # Create a record for this queue type
        record = {
            'Queuetype': queue_type,
            'Memory Allocated': avg_peak_memory,
            'Files Processed': len(peak_memories),
            'Individual Peaks': peak_memories
        }
        return record
    else:
        print(f"No valid peak memory data found for {queue_type}")
        return None

def process_root_folder(root_folder, selected_queues=None):
    """
    Process a root folder containing subfolders for each queue type
    """
    all_queue_data = []
    
    # Get all subdirectories in the root folder
    subfolders = [f.path for f in os.scandir(root_folder) if f.is_dir()]
    
    if not subfolders:
        print(f"No subfolders found in {root_folder}")
        return pd.DataFrame()
    
    print(f"Found {len(subfolders)} queue type folders to process")
    
    for folder in subfolders:
        # Extract queue type from folder name
        queue_type = os.path.basename(folder)
        
        # Skip if not in selected_queues (if specified)
        if selected_queues and queue_type not in selected_queues:
            print(f"Skipping queue type: {queue_type} (not selected)")
            continue
            
        print(f"Processing queue type: {queue_type}")
        
        # Process all mem* files in the subfolder and get average peak memory
        queue_data = process_queue_folder(folder, queue_type)
        
        if queue_data:
            all_queue_data.append(queue_data)
    
    if all_queue_data:
        return pd.DataFrame(all_queue_data)
    else:
        return pd.DataFrame()

def plot_peak_memory_bar_graph(data):
    """
    Create a bar graph showing average peak memory usage for each queue type
    """
    # Sort by peak memory for better visualization
    data = data.sort_values('Memory Allocated', ascending=False)
    
    # Create the figure
    plt.figure(figsize=(12, 6))
    
    # Set style
    sns.set_style("whitegrid")
    
    # Create bar plot
    bars = plt.bar(
        x=data['Queuetype'],
        height=data['Memory Allocated'],
        width=0.7,
        color=sns.color_palette("viridis", len(data))
    )
    
    # Customize plot
    plt.title('Mean Peak Memory Allocated by Queue', fontsize=16)
    plt.xlabel('Queue', fontsize=14)
    plt.ylabel('Mean Peak Memory Allocated (bytes)', fontsize=14)
    
    original_queue_names = data['Queuetype'].tolist()
    translated_names = [name_translator.get(name, name) for name in original_queue_names]
    
    # Set x-tick alignment to right - this makes the labels end at the middle of the bar
    ax = plt.gca()
    ax.set_xticklabels(translated_names, fontsize=12, 
                       ha='right', rotation=45 if len(data) > 6 else 0)
    
    # Set log scale for y-axis
    plt.yscale('log')
    
    # Explicitly import and set minor tick locators
    from matplotlib.ticker import LogLocator, NullLocator
    
    # Add minor ticks to y-axis only (more control)
    ax.yaxis.set_minor_locator(LogLocator(subs=np.arange(2, 10)))
    
    # Turn off minor ticks on x-axis (since it's categorical)
    ax.xaxis.set_minor_locator(NullLocator())
    
    # Make sure minor ticks are visible
    ax.tick_params(which='minor', length=4, color='gray')
    
    # Add grid for better readability (both major and minor)
    plt.grid(axis='y', which='major', linestyle='-', alpha=0.7)
    plt.grid(axis='y', which='minor', linestyle=':', alpha=0.7)
    
    # Adjust layout
    plt.tight_layout()
    filename = f"mem_plot.pdf"
    plt.savefig(filename, format='pdf', bbox_inches='tight', dpi=1200)
    return data

def main():
    # Set up argument parser
    parser = argparse.ArgumentParser(description='Analyze average peak memory usage across different queue types')
    parser.add_argument('root_folder', help='Path to the root folder containing queue type subfolders')
    parser.add_argument('--queues', nargs='+', help='Specific queue types to analyze (separated by space)')
    parser.add_argument('--list-available', action='store_true', help='List all available queue types without processing')
    args = parser.parse_args()
    
    # Get all available queue types
    available_queues = [os.path.basename(f.path) for f in os.scandir(args.root_folder) if f.is_dir()]
    
    # If list-available flag is set, just list the available queues and exit
    if args.list_available:
        print("Available queue types:")
        for queue in sorted(available_queues):
            print(f"  - {queue}")
        return
    
    # If queues are specified, validate them
    if args.queues:
        invalid_queues = [q for q in args.queues if q not in available_queues]
        if invalid_queues:
            print(f"Warning: The following specified queue types were not found: {', '.join(invalid_queues)}")
            print(f"Available queue types are: {', '.join(sorted(available_queues))}")
            valid_queues = [q for q in args.queues if q in available_queues]
            if not valid_queues:
                print("No valid queue types specified. Exiting.")
                return
            print(f"Proceeding with valid queue types: {', '.join(valid_queues)}")
            args.queues = valid_queues
    
    # Process the root folder with selected queues (if specified)
    combined_data = process_root_folder(args.root_folder, args.queues)
    
    if combined_data.empty:
        print("No data to analyze. Please check your input files and folder structure.")
        return
    
    print(f"\nAnalyzing data for {combined_data['Queuetype'].nunique()} queue types")
    
    # Plot peak memory bar graph
    peak_memory = plot_peak_memory_bar_graph(combined_data)
    
    print("\nMean peak memory usage by queue type:")
    for _, row in peak_memory.iterrows():
        print(f"{row['Queuetype']}: {int(row['Memory Allocated']):,} bytes (from {int(row['Files Processed'])} files)")
    
    print("\nDisplaying interactive plot...")
    # plt.show()  # Show plot interactively

if __name__ == "__main__":
    main()
