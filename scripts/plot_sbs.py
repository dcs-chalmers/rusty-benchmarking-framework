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
        "atomic_queue" : "atomic_queue::Queue",
        "basic_queue" : "BasicQueue",
        "bounded_ringbuffer" : "Bounded Ringbuffer",
        "bounded_concurrent_queue" : "Bounded",
        "unbounded_concurrent_queue" : "Unbounded",
        "lf_queue" : "lf_queue::Queue",
        "lockfree_queue" : "lockfree::Queue",
        "lockfree_stack" : "lockfree::Stack",
        "scc2_queue" : "scc2::Queue",
        "scc2_stack" : "scc2::Stack",
        "scc_queue" : "scc::Queue",
        "scc_stack" : "scc::Stack",
        "boost_cpp" : "boost (C++)",
        "faaa_queue_rust" : "Rust FAAAQueue",
        "tz_queue_hp" : "TsigasZhang (HP)",
        "bbq" : "Bbq",
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

def plot_six_subplots(folder_data_list, tau_values, right_queues, highlight_queues=None, output=None, plot_fairness=False):
    """
    Create 6 subplots (2 per folder) with a shared legend.
    
    Args:
        folder_data_list: List of 3 dataframes, one for each folder
        tau_values: List of 3 tau values corresponding to each folder
        right_queues: List of queue types to plot on the right side (selected queues)
        highlight_queues: List of queue types to highlight in the right plots
        output: Output file path for saving the plot
        plot_fairness: If True, plot fairness instead of throughput
    """
    
    # Get all unique queue types from all folders
    all_queue_types = sorted(set().union(*[df['Queuetype'].unique() for df in folder_data_list]))
    
    # Simple color and style assignment based on matplotlib defaults
    # This will give us consistent, nice-looking colors automatically
    colors = plt.cm.tab10.colors + plt.cm.tab20.colors  # Use matplotlib's default color cycles
    line_styles = ['-', '--', '-.', ':']
    marker_styles = ['o', 's', 'D', '^', 'v', '<', '>', 'p', '*', 'h', 'H', 'x', '+']
    
    # Create style mapping - each queue type gets a unique combination
    queue_style_map = {}
    for i, qtype in enumerate(all_queue_types):
        queue_style_map[qtype] = {
            'color': colors[i % len(colors)],
            'line_style': line_styles[i % len(line_styles)],
            'marker_style': marker_styles[i % len(marker_styles)]
        }
    queue_style_map["lcrq_rust"]["color"] = "orange"
    queue_style_map["lprq_rust"]["color"] = "red"
    queue_style_map["faaa_queue_rust"]["color"] = "blue"
    # Calculate complementary queue types for the left plots
    left_queues = [q for q in all_queue_types if q not in right_queues]
    
    # Handle highlight_queues (convert None to empty list for easier processing)
    if highlight_queues is None:
        highlight_queues = []
    
    # Create the figure with 3 rows and 2 columns (6 subplots total)
    fig, axes = plt.subplots(3, 2, figsize=(9, 9), sharey=True)
    
    # Pre-create legend entries for consistent styling
    legend_lines = []
    legend_labels = []
    
    # Add entries for left plot queues (non-highlighted)
    for qtype in sorted(left_queues):
        if qtype in name_translator:
            label = name_translator[qtype]
        else:
            label = qtype
        
        # Create a dummy line with the correct style for the legend
        line_style = queue_style_map[qtype]['line_style']
        marker_style = queue_style_map[qtype]['marker_style']
        color = queue_style_map[qtype]['color']
        
        dummy_line = plt.Line2D([0], [0], 
                               linestyle=line_style, 
                               marker=marker_style,
                               color=color,
                               label=label)
        legend_lines.append(dummy_line)
        legend_labels.append(label)
    
    # Add entries for right plot queues that aren't in left plots (non-highlighted)
    right_only_queues = [q for q in right_queues if q not in left_queues]
    for qtype in sorted(right_only_queues):
        if qtype not in highlight_queues:  # Only add if not highlighted
            if qtype in name_translator:
                label = name_translator[qtype]
            else:
                label = qtype
            
            line_style = queue_style_map[qtype]['line_style']
            marker_style = queue_style_map[qtype]['marker_style']
            color = queue_style_map[qtype]['color']
            
            dummy_line = plt.Line2D([0], [0], 
                                   linestyle=line_style, 
                                   marker=marker_style,
                                   color=color,
                                   label=label,
                                   alpha=0.3,
                                   linewidth=0.5)
            legend_lines.append(dummy_line)
            legend_labels.append(label)
    
    # Add entries for highlighted queues
    for qtype in sorted(highlight_queues):
        if qtype in name_translator:
            label = f"{name_translator[qtype]} ★"
        else:
            label = f"{qtype} ★"
        
        line_style = queue_style_map[qtype]['line_style']
        marker_style = queue_style_map[qtype]['marker_style']
        color = queue_style_map[qtype]['color']
        
        dummy_line = plt.Line2D([0], [0], 
                               linestyle=line_style, 
                               marker=marker_style,
                               color=color,
                               label=label,
                               linewidth=1.5)
        legend_lines.append(dummy_line)
        legend_labels.append(label)
    
    # Set up metric and y-axis settings
    if plot_fairness:
        metric = 'Fairness'
        ylabel = 'Fairness'
        use_log_scale = False
    else:
        metric = 'Throughput'
        ylabel = 'Throughput'
        use_log_scale = True
    
    # Process each folder (row)
    for row_idx, (df, tau) in enumerate(zip(folder_data_list, tau_values)):
        subfolders = df['Subfolder'].unique()
        queue_types = df['Queuetype'].unique()
        
        # Left subplot (all queues not in right_queues)
        ax_left = axes[row_idx, 0]
        ax_left.tick_params(axis='x', labelsize=10)
        ax_left.tick_params(axis='y', labelsize=12)
        ax_left.set_ylabel(ylabel, fontsize="12")
        ax_left.set_xlabel('Thread Count', fontsize="12")
        if use_log_scale:
            ax_left.set_yscale('log')
        ax_left.grid(True)
        ax_left.set_xticks([2, 6, 10, 14, 18, 22, 26, 30, 34, 36])
        
        # Right subplot (all queues, but non-highlighted ones are grayed out)
        ax_right = axes[row_idx, 1]
        ax_right.tick_params(axis='x', labelsize=10)
        ax_right.tick_params(axis='y', labelsize=12)
        ax_right.set_xlabel('Thread Count', fontsize="12")
        if use_log_scale:
            ax_right.set_yscale('log')
        ax_right.grid(True)
        ax_right.set_xticks([2, 6, 10, 14, 18, 22, 26, 30, 34, 36])
        
        # Add tau label on the right side of the rightmost plots only
        ax_right.text(1.02, 0.5, f"τ = {tau}", transform=ax_right.transAxes, 
                     rotation=90, va='center', ha='left', fontsize=14)
        
        # Plot left subplot - All queues not in right_queues with full colors
        for subfolder in subfolders:
            subfolder_data = df[df['Subfolder'] == subfolder]
            for qtype in left_queues:
                if qtype not in queue_types:
                    continue
                    
                queue_data = subfolder_data[subfolder_data['Queuetype'] == qtype]
                if queue_data.empty:
                    continue
                    
                queue_data = queue_data.sort_values('Thread Count')
                
                # Use consistent styling for each queue type
                line_style = queue_style_map[qtype]['line_style']
                marker_style = queue_style_map[qtype]['marker_style']
                color = queue_style_map[qtype]['color']
                
                line, = ax_left.plot(
                    queue_data['Thread Count'],
                    queue_data[metric],
                    marker=marker_style,
                    linestyle=line_style,
                    color=color,
                    markevery=1,
                    linewidth=0.5
                )
        
        # Plot right subplot - First plot non-highlighted queues (grayed out)
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
                color = queue_style_map[qtype]['color']
                
                line, = ax_right.plot(
                    queue_data['Thread Count'],
                    queue_data[metric],
                    marker=marker_style,
                    linestyle=line_style,
                    color='gray',  # Override color for non-highlighted
                    markevery=1,
                    alpha=0.3,  # Reduced opacity for non-highlighted queues
                    linewidth=1,
                )
        
        # Then plot highlighted queues in the right plot
        for subfolder in subfolders:
            subfolder_data = df[df['Subfolder'] == subfolder]
            for qtype in highlight_queues:
                if qtype not in queue_types:
                    continue
                    
                queue_data = subfolder_data[subfolder_data['Queuetype'] == qtype]
                if queue_data.empty:
                    continue
                    
                queue_data = queue_data.sort_values('Thread Count')
                
                # Use consistent styling for each queue type
                line_style = queue_style_map[qtype]['line_style']
                marker_style = queue_style_map[qtype]['marker_style']
                color = queue_style_map[qtype]['color']
                
                line, = ax_right.plot(
                    queue_data['Thread Count'],
                    queue_data[metric],
                    marker=marker_style,
                    linestyle=line_style,
                    color=color,
                    markevery=1,
                    linewidth=1.5,  # Thicker lines for highlighted queues
                    zorder=10,  # Ensure highlighted queues are drawn on top
                )
    
    # Create shared legend at the bottom using pre-created legend entries
    fig.legend(
        legend_lines, legend_labels,
        fontsize='12',
        loc='lower center', 
        bbox_to_anchor=(0.5, 0.0),
        ncol=4,
        frameon=True,
        fancybox=True,
        shadow=True
    )
    
    # Adjust layout to make room for the legend
    plt.tight_layout()
    plt.subplots_adjust(bottom=0.25, left=0.08)
    
    # Save the figure
    if output:
        fig.savefig(output, format='pdf', bbox_inches='tight', dpi=1200)
    else:
        if plot_fairness:
            fig.savefig('six_subplot_comparison_fairness.pdf', format='pdf', bbox_inches='tight', dpi=300)
        else:
            fig.savefig('six_subplot_comparison.pdf', format='pdf', bbox_inches='tight', dpi=300)

def main():
    parser = argparse.ArgumentParser(description='Process and plot benchmark data with 6 subplots (2 per folder).')
    parser.add_argument('folder1', help='First folder containing subfolders with benchmark CSV files')
    parser.add_argument('folder2', help='Second folder containing subfolders with benchmark CSV files')
    parser.add_argument('folder3', help='Third folder containing subfolders with benchmark CSV files')
    parser.add_argument('--tau1', required=True, help='Tau value for first folder')
    parser.add_argument('--tau2', required=True, help='Tau value for second folder')
    parser.add_argument('--tau3', required=True, help='Tau value for third folder')
    parser.add_argument('--output', help='Output file path for saving the plot (optional)')
    parser.add_argument('--right', nargs='+', required=True, help='Queue types to include on the right side')
    parser.add_argument('--highlight', nargs='+', help='Queues to highlight in the right plots (optional)')
    parser.add_argument('--fairness', action='store_true', help='Plot fairness instead of throughput')
    args = parser.parse_args()
    
    # Validate folder paths
    folders = [args.folder1, args.folder2, args.folder3]
    for folder in folders:
        if not os.path.isdir(folder):
            print(f"Error: {folder} is not a valid directory")
            sys.exit(1)
    
    # Load data from all three folders
    folder_data_list = []
    tau_values = [args.tau1, args.tau2, args.tau3]
    
    for i, folder in enumerate(folders):
        print(f"Loading data from folder {i+1}: {folder}")
        df = load_csv_files_by_subfolder(folder)
        if df is None:
            print(f"No valid data was loaded from {folder}.")
            sys.exit(1)
        
        print(f"Loaded data from {len(df['Subfolder'].unique())} subfolders in {folder}")
        print(f"Queue types found in {folder}: {df['Queuetype'].unique()}")
        
        # Process data for thread count
        processed_df = process_data(df, 'Thread Count')
        folder_data_list.append(processed_df)
    
    # Create six subplot comparison
    if args.fairness:
        print("Output can be found in six_subplot_comparison_fairness.pdf")
    else:
        print("Output can be found in six_subplot_comparison.pdf")
    
    plot_six_subplots(
        folder_data_list,
        tau_values,
        args.right, 
        args.highlight, 
        args.output,
        args.fairness
    )

if __name__ == "__main__":
    main()
