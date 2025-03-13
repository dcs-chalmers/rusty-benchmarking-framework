import pandas as pd
import matplotlib.pyplot as plt
import glob
import os
import sys
import argparse


should_pad_data = False


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


def plot_thread_count_results(df):
    """Create plots for each metric with all Queuetypes and Subfolders as
    separate lines."""
    metrics = ["Throughput", "Fairness", "Enqueues", "Dequeues"]
    titles = [
        "Throughput vs. Thread Count",
        "Fairness vs. Thread Count",
        "Number of Enqueues vs. Thread Count",
        "Number of Dequeues vs. Thread Count"
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
                queue_data = subfolder_data[subfolder_data['Queuetype'] == qtype]
                if queue_data.empty:
                    continue

                queue_data = queue_data.sort_values('Thread Count')

                # Cycle through line styles and marker styles
                line_style = line_styles[line_count % len(line_styles)]
                marker_style = marker_styles[line_count % len(marker_styles)]
                line_count += 1

                label = f"{qtype}"
                axes[i].plot(
                    queue_data['Thread Count'],
                    queue_data[metric],
                    marker=marker_style,
                    linestyle=line_style,
                    label=label,
                    markevery=1,
                )

        axes[i].set_title(title)
        axes[i].set_xlabel('Thread Count')
        axes[i].set_ylabel(metric)
        axes[i].set_yscale('log')
        axes[i].grid(True)

        # Create a more compact legend with smaller font
        axes[i].legend(fontsize='small', loc='best')

    plt.tight_layout()
    plt.savefig('thread_count_benchmark.png', dpi=300)
    plt.show()


def plot_mpsc_results(df):
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
                queue_data = subfolder_data[subfolder_data['Queuetype'] == qtype]
                if queue_data.empty:
                    continue

                queue_data = queue_data.sort_values('Producers')

                # Cycle through line styles and marker styles
                line_style = line_styles[line_count % len(line_styles)]
                marker_style = marker_styles[line_count % len(marker_styles)]
                line_count += 1

                label = f"{qtype}"
                axes[i].plot(
                    queue_data['Producers'],
                    queue_data[metric],
                    marker=marker_style,
                    linestyle=line_style,
                    label=label,
                    markevery=5,
                )

        axes[i].set_title(title)
        axes[i].set_xlabel('Producers')
        axes[i].set_ylabel(metric)
        axes[i].set_yscale('log')
        axes[i].grid(True)
        # Create a more compact legend with smaller font
        axes[i].legend(fontsize='small', loc='best')

    plt.tight_layout()
    plt.savefig('mpsc_benchmark.png', dpi=300)
    plt.show()


def plot_spmc_results(df):
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
                queue_data = subfolder_data[subfolder_data['Queuetype'] == qtype]
                if queue_data.empty:
                    continue
                queue_data = queue_data.sort_values('Consumers')

                # Cycle through line styles and marker styles
                line_style = line_styles[line_count % len(line_styles)]
                marker_style = marker_styles[line_count % len(marker_styles)]
                line_count += 1

                label = f"{qtype}"
                axes[i].plot(
                    queue_data['Consumers'],
                    queue_data[metric],
                    marker=marker_style,
                    linestyle=line_style,
                    label=label,
                    markevery=5,
                )

        axes[i].set_title(title)
        axes[i].set_xlabel('Consumers')
        axes[i].set_ylabel(metric)
        axes[i].set_yscale('log')
        axes[i].grid(True)

        # Create a more compact legend with smaller font
        axes[i].legend(fontsize='small', loc='best')

    plt.tight_layout()
    plt.savefig('spmc_benchmark.png', dpi=300)
    plt.show()


def main():
    parser = argparse.ArgumentParser(description='Process and plot benchmark data by subfolder.')
    parser.add_argument('folder', help='Main folder containing subfolders with benchmark CSV files')
    parser.add_argument('plot_type', choices=['spmc', 'thread_count', 'mpsc'],
                        help='What type of benchmark to plot')
    parser.add_argument('--output', help='Output file path for saving the plot (optional)')
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
        plot_spmc_results(processed_df)
    elif args.plot_type == 'thread_count':
        processed_df = process_data(df, 'Thread Count')
        plot_thread_count_results(processed_df)
    elif args.plot_type == 'mpsc':
        processed_df = process_data(df, 'Producers')
        plot_mpsc_results(processed_df)


if __name__ == "__main__":
    main()
