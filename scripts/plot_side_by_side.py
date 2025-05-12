import pandas as pd
import matplotlib.pyplot as plt
import glob
import os
import sys
import argparse
import numpy as np

should_pad_data = False

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
        "bbq" : "BBQ",
        "ms_queue" : "MSQueue",
        }

def load_csv_files_by_subfolder(folder_path):
    """Load CSV files from the given folder path, organized by subfolder."""
    # Get all immediate subfolders
    subfolders = [f.path for f in os.scandir(folder_path) if f.is_dir()]

    if not subfolders:
        print(f"No subfolders found in: {folder_path}")
        return None

    # Dictionary to store dataframes by subfolder
    dfs_by_subfolder = {}

    for subfolder in subfolders:
        subfolder_name = os.path.basename(subfolder)
        csv_files = [file for file in glob.glob(
            os.path.join(subfolder, "**", "*"),
            recursive=True) if not file.endswith('.txt')]

        if not csv_files:
            print(f"No CSV files found in subfolder: {subfolder_name}")
            continue
        subfolder_dfs = []
        for file in csv_files:
            try:
                if os.path.isfile(file):
                    df = pd.read_csv(file)
                    # Add a column to identify the subfolder
                    df['Subfolder'] = subfolder_name
                    subfolder_dfs.append(df)
            except Exception as e:
                print(f"Error reading {file}: {e}")

        if subfolder_dfs:
            dfs_by_subfolder[subfolder_name] = pd.concat(subfolder_dfs,
                                                         ignore_index=True)

    if not dfs_by_subfolder:
        return None

    # Combine all dataframes with subfolder identification
    return pd.concat(dfs_by_subfolder.values(), ignore_index=True)

def process_data(df, group_by):
    """Average results grouped by Test ID,
    group_by, Queuetype, and Subfolder."""
    grouped = df.groupby(['Test ID', group_by, 'Queuetype', 'Subfolder']).agg({
        'Throughput': 'mean',
        'Fairness': 'mean',
        'Enqueues': 'mean',
        'Dequeues': 'mean'
    }).reset_index()
    return grouped

def plot_side_by_side_fairness(df, right_queues, highlight_queues=None, output=None):
    """
    Create side-by-side plots with a shared legend.
    
    Args:
        df: The dataframe containing the data
        right_queues: List of queue types to plot on the right side (selected queues)
        highlight_queues: List of queue types to highlight in the right plot
        output: Output file path for saving the plot
    """
    metrics = ["Fairness"]
    titles = ["Rust ecosystem and C++ queues", "Our queues"]
    
    # Define a set of line styles and marker styles for better distinction
    line_styles = ['-', '--', '-.', ':']
    marker_styles = ['o', 's', 'D', '^', 'v', '<', '>', 'p', '*', 'h', 'H', 'x', '+']
    
    # Define a fixed, predetermined order for queue types to ensure consistent styling
    known_queue_types = [
        "faaaq_rust_optimised",
        "faaaq_rust_unoptimised",
        "faaa_queue_cpp",
        "lprq_rust_correct",
        "lcrq_rust_correct",
        "lcrq_cpp",
        "lprq_rust_unoptimised",
        "lcrq_rust_unoptimised",
        "lprq_cpp",
        "lcrq_rust",
        "lprq_rust",
        "moodycamel_cpp",
        "seg_queue",
        "array_queue",
        "atomic_queue",
        "basic_queue",
        "bounded_ringbuffer",
        "bounded_concurrent_queue",
        "unbounded_concurrent_queue",
        "lf_queue",
        "lockfree_queue",
        "lockfree_stack",
        "scc2_queue",
        "scc2_stack",
        "scc_queue",
        "scc_stack",
        "boost_cpp",
        "faaa_queue_rust",
        "tz_queue_hp",
        "bbq",
        "ms_queue",
    ]
    
    # Create a mapping of queue types to styles based on the fixed order
    queue_style_map = {}
    for i, qtype in enumerate(known_queue_types):
        queue_style_map[qtype] = {
            'line_style': line_styles[i % len(line_styles)],
            'marker_style': marker_styles[i % len(marker_styles)]
        }
    
    # Handle any queue types not in the predetermined list
    all_queue_types = df['Queuetype'].unique()
    unknown_types = [q for q in all_queue_types if q not in queue_style_map]
    for i, qtype in enumerate(unknown_types):
        # Use a different starting point to avoid style conflicts with known types
        style_idx = len(known_queue_types) + i
        queue_style_map[qtype] = {
            'line_style': line_styles[style_idx % len(line_styles)],
            'marker_style': marker_styles[style_idx % len(marker_styles)]
        }
    
    # Use a different color for each subfolder
    subfolders = df['Subfolder'].unique()
    queue_types = df['Queuetype'].unique()
    
    # Calculate complementary queue types for the left plot
    left_queues = [q for q in queue_types if q not in right_queues]
    
    # Handle highlight_queues (convert None to empty list for easier processing)
    if highlight_queues is None:
        highlight_queues = []
    
    # Create the figure with two subplots side by side
    # Increase the figure width to provide more space
    fig, axes = plt.subplots(1, 2, figsize=(12, 8), sharey=True)
    
    # Store all plot line objects and their labels for the combined legend
    all_lines = []
    all_labels = []
    
    # Common settings for both plots
    for ax, title in zip(axes, titles):
        ax.set_xlabel('Thread Count')
        ax.grid(True)
        ax.set_xticks([2, 6, 10, 14, 18, 22, 26, 30, 34, 36])
        ax.set_title(title)
    
    # Left subplot - All queues not in right_queues with full colors
    for subfolder in subfolders:
        subfolder_data = df[df['Subfolder'] == subfolder]
        for qtype in left_queues:
            queue_data = subfolder_data[subfolder_data['Queuetype'] == qtype]
            if queue_data.empty:
                continue
                
            queue_data = queue_data.sort_values('Thread Count')
            
            # Use consistent styling for each queue type
            line_style = queue_style_map[qtype]['line_style']
            marker_style = queue_style_map[qtype]['marker_style']
            
            if qtype in name_translator:
                label = f"{name_translator[qtype]}"
            else:
                label = f"{qtype}"
                
            line, = axes[0].plot(
                queue_data['Thread Count'],
                queue_data['Fairness'],
                marker=marker_style,
                linestyle=line_style,
                label=label,
                markevery=1,
            )
            
            # Add to the legend
            all_lines.append(line)
            all_labels.append(label)
    
    # Right subplot - Plot all queues, but non-highlighted ones are grayed out
    # First plot non-highlighted queues (grayed out)
    for subfolder in subfolders:
        subfolder_data = df[df['Subfolder'] == subfolder]
        for qtype in queue_types:
            # Skip highlighted queues for now
            if qtype in highlight_queues:
                continue
                
            queue_data = subfolder_data[subfolder_data['Queuetype'] == qtype]
            if queue_data.empty:
                continue
                
            queue_data = queue_data.sort_values('Thread Count')
            
            # Use consistent styling for each queue type
            line_style = queue_style_map[qtype]['line_style']
            marker_style = queue_style_map[qtype]['marker_style']
            
            # No need to add to legend if already in left plot
            if qtype in left_queues:
                label = None
            else:
                if qtype in name_translator:
                    label = f"{name_translator[qtype]}"
                else:
                    label = f"{qtype}"
                
            line, = axes[1].plot(
                queue_data['Thread Count'],
                queue_data['Fairness'],
                marker=marker_style,
                linestyle=line_style,
                label=label,
                markevery=1,
                alpha=0.3,  # Reduced opacity for non-highlighted queues
                color='gray',
                linewidth=1,
            )
            
            # Add to legend only if it's not in the left plot
            if label is not None:
                all_lines.append(line)
                all_labels.append(label)
    
    # Then plot highlighted queues in the right plot
    for subfolder in subfolders:
        subfolder_data = df[df['Subfolder'] == subfolder]
        for qtype in highlight_queues:
            queue_data = subfolder_data[subfolder_data['Queuetype'] == qtype]
            if queue_data.empty:
                continue
                
            queue_data = queue_data.sort_values('Thread Count')
            
            # Use consistent styling for each queue type
            line_style = queue_style_map[qtype]['line_style']
            marker_style = queue_style_map[qtype]['marker_style']
            
            if qtype in name_translator:
                label = f"{name_translator[qtype]} ★"  # Add star to highlight in legend
            else:
                label = f"{qtype} ★"
                
            line, = axes[1].plot(
                queue_data['Thread Count'],
                queue_data['Fairness'],
                marker=marker_style,
                linestyle=line_style,
                label=label,
                markevery=1,
                linewidth=2.5,  # Thicker lines for highlighted queues
                zorder=10,  # Ensure highlighted queues are drawn on top
            )
            
            # Always add highlighted queues to the legend
            all_lines.append(line)
            all_labels.append(label)
    
    # Add common y-axis label
    # Increase the padding to prevent overlap with tick labels
    fig.text(0.01, 0.5, 'Fairness', va='center', rotation='vertical', fontsize=12)
    
    # Create shared legend at the bottom
    fig.legend(
        all_lines, all_labels,
        fontsize='large',
        loc='lower center', 
        bbox_to_anchor=(0.5, 0.0),
        ncol=4,
        frameon=True,
        fancybox=True,
        shadow=True
    )
    
    # Adjust layout to make room for the legend and y-axis label
    plt.tight_layout()
    # Increase left margin to make room for y-axis label
    plt.subplots_adjust(bottom=0.2, left=0.1)
    
    # Save the figure if output is specified
    if output:
        fig.savefig(output, format='pdf', bbox_inches='tight', dpi=300)
    else:
        fig.savefig('side_by_side_comparison_fairness.pdf', format='pdf', bbox_inches='tight', dpi=300)
    
    # Display the plot
    plt.show()
    
def plot_side_by_side(df, right_queues, highlight_queues=None, output=None):
    """
    Create side-by-side plots with a shared legend.
    
    Args:
        df: The dataframe containing the data
        right_queues: List of queue types to plot on the right side (selected queues)
        highlight_queues: List of queue types to highlight in the right plot
        output: Output file path for saving the plot
    """
    metrics = ["Throughput"]
    titles = ["Rust ecosystem and C++ queues", "Our queues"]
    
    # Define a set of line styles and marker styles for better distinction
    line_styles = ['-', '--', '-.', ':']
    marker_styles = ['o', 's', 'D', '^', 'v', '<', '>', 'p', '*', 'h', 'H', 'x', '+']
    
    # Define a fixed, predetermined order for queue types to ensure consistent styling
    known_queue_types = [
        "faaaq_rust_optimised",
        "faaaq_rust_unoptimised",
        "faaa_queue_cpp",
        "lprq_rust_correct",
        "lcrq_rust_correct",
        "lcrq_cpp",
        "lprq_rust_unoptimised",
        "lcrq_rust_unoptimised",
        "lprq_cpp",
        "lcrq_rust",
        "lprq_rust",
        "moodycamel_cpp",
        "seg_queue",
        "array_queue",
        "atomic_queue",
        "basic_queue",
        "bounded_ringbuffer",
        "bounded_concurrent_queue",
        "unbounded_concurrent_queue",
        "lf_queue",
        "lockfree_queue",
        "lockfree_stack",
        "scc2_queue",
        "scc2_stack",
        "scc_queue",
        "scc_stack",
        "boost_cpp",
        "faaa_queue_rust",
        "tz_queue_hp",
        "bbq",
        "ms_queue",
    ]
    
    # Create a mapping of queue types to styles based on the fixed order
    queue_style_map = {}
    for i, qtype in enumerate(known_queue_types):
        queue_style_map[qtype] = {
            'line_style': line_styles[i % len(line_styles)],
            'marker_style': marker_styles[i % len(marker_styles)]
        }
    
    # Handle any queue types not in the predetermined list
    all_queue_types = df['Queuetype'].unique()
    unknown_types = [q for q in all_queue_types if q not in queue_style_map]
    for i, qtype in enumerate(unknown_types):
        # Use a different starting point to avoid style conflicts with known types
        style_idx = len(known_queue_types) + i
        queue_style_map[qtype] = {
            'line_style': line_styles[style_idx % len(line_styles)],
            'marker_style': marker_styles[style_idx % len(marker_styles)]
        }
    
    # Use a different color for each subfolder
    subfolders = df['Subfolder'].unique()
    queue_types = df['Queuetype'].unique()
    
    # Calculate complementary queue types for the left plot
    left_queues = [q for q in queue_types if q not in right_queues]
    
    # Handle highlight_queues (convert None to empty list for easier processing)
    if highlight_queues is None:
        highlight_queues = []
    
    # Create the figure with two subplots side by side
    # Increase the figure width to provide more space
    fig, axes = plt.subplots(1, 2, figsize=(12, 8), sharey=True)
    
    # Store all plot line objects and their labels for the combined legend
    all_lines = []
    all_labels = []
    
    # Common settings for both plots
    for ax, title in zip(axes, titles):
        ax.set_xlabel('Thread Count')
        ax.set_yscale('log')
        ax.grid(True)
        ax.set_xticks([2, 6, 10, 14, 18, 22, 26, 30, 34, 36])
        ax.set_title(title)
    
    # Left subplot - All queues not in right_queues with full colors
    for subfolder in subfolders:
        subfolder_data = df[df['Subfolder'] == subfolder]
        for qtype in left_queues:
            queue_data = subfolder_data[subfolder_data['Queuetype'] == qtype]
            if queue_data.empty:
                continue
                
            queue_data = queue_data.sort_values('Thread Count')
            
            # Use consistent styling for each queue type
            line_style = queue_style_map[qtype]['line_style']
            marker_style = queue_style_map[qtype]['marker_style']
            
            if qtype in name_translator:
                label = f"{name_translator[qtype]}"
            else:
                label = f"{qtype}"
                
            line, = axes[0].plot(
                queue_data['Thread Count'],
                queue_data['Throughput'],
                marker=marker_style,
                linestyle=line_style,
                label=label,
                markevery=1,
            )
            
            # Add to the legend
            all_lines.append(line)
            all_labels.append(label)
    
    # Right subplot - Plot all queues, but non-highlighted ones are grayed out
    # First plot non-highlighted queues (grayed out)
    for subfolder in subfolders:
        subfolder_data = df[df['Subfolder'] == subfolder]
        for qtype in queue_types:
            # Skip highlighted queues for now
            if qtype in highlight_queues:
                continue
                
            queue_data = subfolder_data[subfolder_data['Queuetype'] == qtype]
            if queue_data.empty:
                continue
                
            queue_data = queue_data.sort_values('Thread Count')
            
            # Use consistent styling for each queue type
            line_style = queue_style_map[qtype]['line_style']
            marker_style = queue_style_map[qtype]['marker_style']
            
            # No need to add to legend if already in left plot
            if qtype in left_queues:
                label = None
            else:
                if qtype in name_translator:
                    label = f"{name_translator[qtype]}"
                else:
                    label = f"{qtype}"
                
            line, = axes[1].plot(
                queue_data['Thread Count'],
                queue_data['Throughput'],
                marker=marker_style,
                linestyle=line_style,
                label=label,
                markevery=1,
                alpha=0.3,  # Reduced opacity for non-highlighted queues
                color='gray',
                linewidth=1,
            )
            
            # Add to legend only if it's not in the left plot
            if label is not None:
                all_lines.append(line)
                all_labels.append(label)
    
    # Then plot highlighted queues in the right plot
    for subfolder in subfolders:
        subfolder_data = df[df['Subfolder'] == subfolder]
        for qtype in highlight_queues:
            queue_data = subfolder_data[subfolder_data['Queuetype'] == qtype]
            if queue_data.empty:
                continue
                
            queue_data = queue_data.sort_values('Thread Count')
            
            # Use consistent styling for each queue type
            line_style = queue_style_map[qtype]['line_style']
            marker_style = queue_style_map[qtype]['marker_style']
            
            if qtype in name_translator:
                label = f"{name_translator[qtype]} ★"  # Add star to highlight in legend
            else:
                label = f"{qtype} ★"
                
            line, = axes[1].plot(
                queue_data['Thread Count'],
                queue_data['Throughput'],
                marker=marker_style,
                linestyle=line_style,
                label=label,
                markevery=1,
                linewidth=2.5,  # Thicker lines for highlighted queues
                zorder=10,  # Ensure highlighted queues are drawn on top
            )
            
            # Always add highlighted queues to the legend
            all_lines.append(line)
            all_labels.append(label)
    
    # Add common y-axis label
    # Increase the padding to prevent overlap with tick labels
    fig.text(0.01, 0.5, 'Throughput', va='center', rotation='vertical', fontsize=12)
    
    # Create shared legend at the bottom
    fig.legend(
        all_lines, all_labels,
        fontsize='large',
        loc='lower center', 
        bbox_to_anchor=(0.5, 0.0),
        ncol=4,
        frameon=True,
        fancybox=True,
        shadow=True
    )
    
    # Adjust layout to make room for the legend and y-axis label
    plt.tight_layout()
    # Increase left margin to make room for y-axis label
    plt.subplots_adjust(bottom=0.2, left=0.1)
    
    # Save the figure if output is specified
    if output:
        fig.savefig(output, format='pdf', bbox_inches='tight', dpi=300)
    else:
        fig.savefig('side_by_side_comparison.pdf', format='pdf', bbox_inches='tight', dpi=300)
    
    # Display the plot
    plt.show()

def main():
    parser = argparse.ArgumentParser(description='Process and plot benchmark data with side-by-side comparison.')
    parser.add_argument('folder', help='Main folder containing subfolders with benchmark CSV files')
    parser.add_argument('--output', help='Output file path for saving the plot (optional)')
    parser.add_argument('--right', nargs='+', required=True, help='Queue types to include on the right side')
    parser.add_argument('--highlight', nargs='+', help='Queues to highlight in the right plot (optional)')
    args = parser.parse_args()
    
    if not os.path.isdir(args.folder):
        print(f"Error: {args.folder} is not a valid directory")
        sys.exit(1)
        
    df = load_csv_files_by_subfolder(args.folder)
    if df is None:
        print("No valid data was loaded.")
        sys.exit(1)
        
    print(f"Loaded data from {len(df['Subfolder'].unique())} subfolders")
    print(f"Queue types found: {df['Queuetype'].unique()}")
    
    # Process data for thread count
    processed_df = process_data(df, 'Thread Count')
    
    # Create side-by-side plots
    plot_side_by_side_fairness(
        processed_df, 
        args.right, 
        args.highlight, 
        args.output
    )

if __name__ == "__main__":
    main()
