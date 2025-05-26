import pandas as pd
import matplotlib.pyplot as plt
import glob
import os
import sys
import argparse

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
        "wfqueue" : "Wfqueue",
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

def plot_six_subplots_combined(folder_data_list, tau_values, queues=None, ignore_queues=None, output=None):
    """
    Create 6 subplots (3 rows × 2 columns) with Throughput and Fairness for each tau value.
    
    Args:
        folder_data_list: List of 3 dataframes, one for each folder
        tau_values: List of 3 tau values corresponding to each folder
        queues: List of queue types to plot. If None, all queues are plotted.
        ignore_queues: List of queue types to ignore/exclude from the plot.
        output: Output file path for saving the plot
    """
    
    # Get all unique queue types from all folders and sort them
    all_queue_types = sorted(set().union(*[df['Queuetype'].unique() for df in folder_data_list]))
    
    # Simple color and style assignment based on matplotlib defaults
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
    
    # Handle ignore_queues (convert None to empty list for easier processing)
    if ignore_queues is None:
        ignore_queues = []
    
    # Create the figure with 3 rows and 2 columns (6 subplots total)
    fig, axes = plt.subplots(3, 2, figsize=(9, 9))
    
    # Store legend entries
    legend_lines = []
    legend_labels = []
    added_labels = set()
    
    metrics = ["Throughput", "Fairness"]
    
    # Process each folder (row)
    for row_idx, (df, tau) in enumerate(zip(folder_data_list, tau_values)):
        subfolders = df['Subfolder'].unique()
        queue_types = df['Queuetype'].unique()
        
        # Filter queue types based on parameters
        filtered_queue_types = []
        for qtype in queue_types:
            if qtype in ignore_queues:
                continue
            if queues and qtype not in queues:
                continue
            filtered_queue_types.append(qtype)
        
        # Plot both metrics for this tau value
        for col_idx, metric in enumerate(metrics):
            ax = axes[row_idx, col_idx]
            
            # Configure subplot
            ax.tick_params(axis='x', labelsize=12)
            ax.tick_params(axis='y', labelsize=14)
            ax.set_xlabel('Thread Count', fontsize="14")
            ax.set_ylabel(metric, fontsize="14")
            if metric == "Throughput":
                ax.set_yscale('log')
            ax.grid(True)
            ax.set_xticks([2, 6, 10, 14, 18, 22, 26, 30, 34, 36])
            
            # Add tau label on the right side of the Fairness plots only
            if col_idx == 1:  # Fairness column
                ax.text(1.02, 0.5, f"τ = {tau}", transform=ax.transAxes, 
                       rotation=90, va='center', ha='left', fontsize=16)
            
            # Plot all queues for this folder
            for subfolder in subfolders:
                subfolder_data = df[df['Subfolder'] == subfolder]
                for qtype in filtered_queue_types:
                    queue_data = subfolder_data[subfolder_data['Queuetype'] == qtype]
                    if queue_data.empty:
                        continue
                        
                    queue_data = queue_data.sort_values('Thread Count')
                    
                    # Use consistent styling for each queue type
                    line_style = queue_style_map[qtype]['line_style']
                    marker_style = queue_style_map[qtype]['marker_style']
                    color = queue_style_map[qtype]['color']
                    
                    if qtype in name_translator:
                        label = name_translator[qtype]
                    else:
                        label = qtype
                    
                    # Add to legend only once (for the first occurrence)
                    if label not in added_labels:
                        legend_lines.append(plt.Line2D([0], [0], 
                                                      linestyle=line_style, 
                                                      marker=marker_style,
                                                      color=color,
                                                      label=label,
                                                      linewidth=0.5))
                        legend_labels.append(label)
                        added_labels.add(label)
                    
                    ax.plot(
                        queue_data['Thread Count'],
                        queue_data[metric],
                        marker=marker_style,
                        linestyle=line_style,
                        color=color,
                        markevery=1,
                        linewidth=0.5
                    )
    
    # Create shared legend at the bottom using pre-created legend entries
    ncols = max(1, len(legend_lines) // 2)
    if len(legend_lines) % 2 != 0:  # If odd number of handles, add one more column
        ncols += 1
    
    fig.legend(
        legend_lines, legend_labels,
        fontsize='13',
        loc='lower center', 
        bbox_to_anchor=(0.5, 0.0),
        ncol=ncols,
        frameon=True,
        fancybox=True,
        shadow=True,
        columnspacing=1.5,
    )
    
    # Adjust layout to make room for the legend
    plt.tight_layout()
    plt.subplots_adjust(bottom=0.15, left=0.08, right=0.95)
    
    # Save the figure
    if output:
        fig.savefig(output, format='pdf', bbox_inches='tight', dpi=1200)
    else:
        fig.savefig('six_subplot_combined_metrics.pdf', format='pdf', bbox_inches='tight', dpi=1200)

def main():
    parser = argparse.ArgumentParser(description='Process and plot benchmark data with 6 subplots (3 folders × 2 metrics).')
    parser.add_argument('folder1', help='First folder containing subfolders with benchmark CSV files')
    parser.add_argument('folder2', help='Second folder containing subfolders with benchmark CSV files')
    parser.add_argument('folder3', help='Third folder containing subfolders with benchmark CSV files')
    parser.add_argument('--tau1', required=True, help='Tau value for first folder')
    parser.add_argument('--tau2', required=True, help='Tau value for second folder')
    parser.add_argument('--tau3', required=True, help='Tau value for third folder')
    parser.add_argument('--output', help='Output file path for saving the plot (optional)')
    parser.add_argument('--queues', nargs='+', help='Specific queues to plot (optional)')
    parser.add_argument('--ignore', nargs='+', help='Queues to exclude from the plot (optional)')
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
    print("Output can be found in six_subplot_combined_metrics.pdf")
    
    plot_six_subplots_combined(
        folder_data_list,
        tau_values,
        args.queues, 
        args.ignore, 
        args.output
    )

if __name__ == "__main__":
    main()
