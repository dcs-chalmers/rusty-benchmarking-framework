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

def plot_combined_metrics(df, queues=None, highlight_queues=None, ignore_queues=None):
    """Create a single plot window with both Throughput and Fairness metrics.
    
    Args:
        df: The dataframe containing the data
        queues: List of queue types to plot. If None, all queues are plotted.
        highlight_queues: List of queue types to highlight. If None or empty, all queues are displayed normally.
        ignore_queues: List of queue types to ignore/exclude from the plot. Takes precedence over queues and highlight_queues.
    """
    metrics = ["Throughput", "Fairness"]
    y_labels = ["Throughput", "Fairness"]
    
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
    
    # Handle ignore_queues (convert None to empty list for easier processing)
    if ignore_queues is None:
        ignore_queues = []
        
    # Check if we're in highlight mode
    highlight_mode = highlight_queues is not None and len(highlight_queues) > 0
    
    # Create a single figure with two subplots (Throughput and Fairness)
    fig, axs = plt.subplots(1, 2, figsize=(12, 8))
    
    # Keep track of plotted queues for legend
    plotted_queues = {}
    
    for i, (metric, y_label) in enumerate(zip(metrics, y_labels)):
        # First plot non-highlighted queues if in highlight mode
        if highlight_mode:
            # Plot non-highlighted queues first (grayed out)
            for subfolder in subfolders:
                subfolder_data = df[df['Subfolder'] == subfolder]
                for qtype in queue_types:
                    # Skip queue types to ignore
                    if qtype in ignore_queues:
                        continue
                        
                    # Skip queue types not in the specified list
                    if queues and qtype not in queues:
                        continue
                        
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
                    
                    if qtype in name_translator:
                        label = f"{name_translator[qtype]}"
                    else:
                        label = f"{qtype}"
                    
                    # Only add to legend once (for the first metric)
                    if i == 0:
                        plotted_queues[qtype] = {'label': label, 'highlighted': False}
                        
                    axs[i].plot(
                        queue_data['Thread Count'],
                        queue_data[metric],
                        marker=marker_style,
                        linestyle=line_style,
                        label=label if i == 0 else "_nolegend_",  # Only add to legend for the first subplot
                        markevery=1,
                        alpha=0.3,  # Reduced opacity for non-highlighted queues
                        color='gray',
                        linewidth=1,
                    )
            
            # Then plot highlighted queues
            for subfolder in subfolders:
                subfolder_data = df[df['Subfolder'] == subfolder]
                for qtype in highlight_queues:
                    # Skip queue types to ignore
                    if qtype in ignore_queues:
                        continue
                        
                    # Skip queue types not in the specified list
                    if queues and qtype not in queues:
                        continue
                        
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
                    
                    # Only add to legend once (for the first metric)
                    if i == 0:
                        plotted_queues[qtype] = {'label': label, 'highlighted': True}
                        
                    axs[i].plot(
                        queue_data['Thread Count'],
                        queue_data[metric],
                        marker=marker_style,
                        linestyle=line_style,
                        label=label if i == 0 else "_nolegend_",  # Only add to legend for the first subplot
                        markevery=1,
                        linewidth=2.5,  # Thicker lines for highlighted queues
                        zorder=10,  # Ensure highlighted queues are drawn on top
                    )
        else:
            # Normal mode - plot all queues with full colors
            for subfolder in subfolders:
                subfolder_data = df[df['Subfolder'] == subfolder]
                for qtype in queue_types:
                    # Skip queue types to ignore
                    if qtype in ignore_queues:
                        continue
                        
                    # Skip queue types not in the specified list
                    if queues and qtype not in queues:
                        continue
                        
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
                    
                    # Only add to legend once (for the first metric)
                    if i == 0:
                        plotted_queues[qtype] = {'label': label, 'highlighted': False}
                        
                    axs[i].plot(
                        queue_data['Thread Count'],
                        queue_data[metric],
                        marker=marker_style,
                        linestyle=line_style,
                        label=label if i == 0 else "_nolegend_",  # Only add to legend for the first subplot
                        markevery=1,
                    )
        
        # Set subplot titles and axes
        axs[i].set_title(f"{metric} vs. Thread Count", fontsize=14)
        axs[i].set_xticks([2, 6, 10, 14, 18, 22, 26, 30, 34, 36])
        axs[i].set_xlabel('Thread Count')
        axs[i].set_ylabel(metric)
        # Only use log scale for Throughput, not for Fairness
        if metric == "Throughput":
            axs[i].set_yscale('log')
        axs[i].grid(True)
    
    # Check if anything was plotted
    if not plotted_queues:
        print("Warning: No data to plot. Check if specified queues exist in the dataset.")
        plt.close(fig)
        return
    
    # Place the legend below both subplots as two rows
    handles, labels = axs[0].get_legend_handles_labels()
    
    # Calculate columns needed for two rows
    ncols = max(1, len(handles) // 2)
    if len(handles) % 2 != 0:  # If odd number of handles, add one more column
        ncols += 1
    
    fig.legend(
        handles, 
        labels,
        # fontsize='large',  # Large font for better readability
        fontsize=16,  # Explicit larger font size
        loc='upper center',
        bbox_to_anchor=(0.5, 0.08),
        ncol=ncols,  # Set columns for two rows
        frameon=True,
        fancybox=True,
        shadow=True,
        columnspacing=1.5,  # More space between columns
    )
    
    # Adjust layout to make room for the legend
    plt.tight_layout()
    plt.subplots_adjust(bottom=0.15)  # More space for the two-row legend
    
    # Save the figure with fixed dimensions 
    filename = "Combined_Performance_Metrics.pdf"
    fig.savefig(filename, format='pdf', bbox_inches='tight', dpi=1200)
    
    # Display the plot
    # plt.show()

def plot_mpsc_results(df, queues=None, highlight_queues=None, ignore_queues=None):
    """Create plots for each metric with all Queuetypes and Subfolders as separate lines."""
    metrics = ["Throughput", "Fairness", "Enqueues", "Dequeues"]
    titles = [
        "Throughput vs. Producers",
        "Fairness vs. Producers",
        "Number of Enqueues vs. Producers",
        "Number of Dequeues vs. Producers"
    ]

    # Define a set of line styles and marker styles for better distinction
    line_styles = ['-', '--', '-.', ':']
    marker_styles = ['o', 's', 'D', '^', 'v', '<', '>', 'p', '*', 'h', 'H', 'x', '+']

    # Use a different color for each subfolder
    subfolders = df['Subfolder'].unique()
    queue_types = df['Queuetype'].unique()

    # Create a single figure with 4 subplots (one for each metric)
    fig, axes = plt.subplots(2, 2, figsize=(15, 10))
    fig.suptitle('Performance Metrics by Queuetype and Subfolder', fontsize=16)

    # Flatten the axes array for easier indexing
    axes = axes.flatten()

    # Plot all combinations of Queuetypes and Subfolders
    for i, (metric, title) in enumerate(zip(metrics, titles)):
        line_count = 0
        for subfolder in subfolders:
            subfolder_data = df[df['Subfolder'] == subfolder]

            for qtype in queue_types:
                if queues and qtype not in queues:
                    continue
                if ignore_queues and qtype in ignore_queues:
                    continue
                queue_data = subfolder_data[subfolder_data['Queuetype'] == qtype]
                if queue_data.empty:
                    continue

                queue_data = queue_data.sort_values('Producers')

                # Cycle through line styles and marker styles
                line_style = line_styles[line_count % len(line_styles)]
                marker_style = marker_styles[line_count % len(marker_styles)]
                line_count += 1

                if qtype in name_translator:
                    label = f"{name_translator[qtype]}"
                else:
                    label = f"{qtype}"
                
                # Apply highlight styling if applicable
                alpha = 1.0
                linewidth = 1.5
                color = None
                zorder = 5
                
                if highlight_queues and qtype in highlight_queues:
                    label += " ★"
                    linewidth = 2.5
                    zorder = 10
                elif highlight_queues:
                    alpha = 0.3
                    color = 'gray'
                
                axes[i].plot(
                    queue_data['Producers'],
                    queue_data[metric],
                    marker=marker_style,
                    linestyle=line_style,
                    label=label,
                    markevery=5,
                    alpha=alpha,
                    color=color,
                    linewidth=linewidth,
                    zorder=zorder
                )

        axes[i].set_title(title)
        axes[i].set_xlabel('Producers')
        axes[i].set_ylabel(metric)
        axes[i].set_yscale('log')
        axes[i].grid(True)
        # Create a more compact legend with smaller font
        axes[i].legend(fontsize='small', loc='best')

    plt.tight_layout()
    plt.savefig('mpsc_benchmark.pdf', format='pdf', dpi=1200)
    plt.show()

def plot_spmc_results(df, queues=None, highlight_queues=None, ignore_queues=None):
    """Create plots for each metric with all Queuetypes and Subfolders as separate lines."""
    metrics = ["Throughput", "Fairness", "Enqueues", "Dequeues"]
    titles = [
        "Throughput vs. Consumers",
        "Fairness vs. Consumers",
        "Number of Enqueues vs. Consumers",
        "Number of Dequeues vs. Consumers"
    ]

    # Define a set of line styles and marker styles for better distinction
    line_styles = ['-', '--', '-.', ':']
    marker_styles = ['o', 's', 'D', '^', 'v', '<', '>', 'p', '*', 'h', 'H', 'x', '+']

    # Use a different color for each subfolder
    subfolders = df['Subfolder'].unique()
    queue_types = df['Queuetype'].unique()

    # Create a single figure with 4 subplots (one for each metric)
    fig, axes = plt.subplots(2, 2, figsize=(15, 10))
    fig.suptitle('Performance Metrics by Queuetype and Subfolder', fontsize=16)

    # Flatten the axes array for easier indexing
    axes = axes.flatten()

    # Plot all combinations of Queuetypes and Subfolders
    for i, (metric, title) in enumerate(zip(metrics, titles)):
        line_count = 0
        for subfolder in subfolders:
            subfolder_data = df[df['Subfolder'] == subfolder]

            for qtype in queue_types:
                if queues and qtype not in queues:
                    continue
                if ignore_queues and qtype in ignore_queues:
                    continue
                queue_data = subfolder_data[subfolder_data['Queuetype'] == qtype]
                if queue_data.empty:
                    continue
                queue_data = queue_data.sort_values('Consumers')

                # Cycle through line styles and marker styles
                line_style = line_styles[line_count % len(line_styles)]
                marker_style = marker_styles[line_count % len(marker_styles)]
                line_count += 1

                if qtype in name_translator:
                    label = f"{name_translator[qtype]}"
                else:
                    label = f"{qtype}"
                
                # Apply highlight styling if applicable
                alpha = 1.0
                linewidth = 1.5
                color = None
                zorder = 5
                
                if highlight_queues and qtype in highlight_queues:
                    label += " ★"
                    linewidth = 2.5
                    zorder = 10
                elif highlight_queues:
                    alpha = 0.3
                    color = 'gray'
                
                axes[i].plot(
                    queue_data['Consumers'],
                    queue_data[metric],
                    marker=marker_style,
                    linestyle=line_style,
                    label=label,
                    markevery=5,
                    alpha=alpha,
                    color=color,
                    linewidth=linewidth,
                    zorder=zorder
                )

        axes[i].set_title(title)
        axes[i].set_xlabel('Consumers')
        axes[i].set_ylabel(metric)
        axes[i].set_yscale('log')
        axes[i].grid(True)

        # Create a more compact legend with smaller font
        axes[i].legend(fontsize='small', loc='best')

    plt.tight_layout()
    plt.savefig('spmc_benchmark.pdf', format='pdf', dpi=1200)
    plt.show()

def main():
    parser = argparse.ArgumentParser(description='Process and plot benchmark data by subfolder.')
    parser.add_argument('folder', help='Main folder containing subfolders with benchmark CSV files')
    parser.add_argument('plot_type', choices=['spmc', 'thread_count', 'mpsc'],
                        help='What type of benchmark to plot')
    parser.add_argument('--output', help='Output file path for saving the plot (optional)')
    parser.add_argument('--queues', nargs='+', help='Specific queues to plot (optional)')
    parser.add_argument('--highlight', nargs='+', help='Queues to highlight among the plotted ones (optional)')
    parser.add_argument('--ignore', nargs='+', help='Queues to exclude from the plot (optional)')
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
    
    if args.plot_type == 'spmc':
        processed_df = process_data(df, 'Consumers')
        plot_spmc_results(processed_df, args.queues, args.highlight, args.ignore)
    elif args.plot_type == 'thread_count':
        processed_df = process_data(df, 'Thread Count')
        plot_combined_metrics(processed_df, args.queues, args.highlight, args.ignore)
    elif args.plot_type == 'mpsc':
        processed_df = process_data(df, 'Producers')
        plot_mpsc_results(processed_df, args.queues, args.highlight, args.ignore)

if __name__ == "__main__":
    main()
