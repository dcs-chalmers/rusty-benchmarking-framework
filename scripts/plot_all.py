import pandas as pd
import matplotlib.pyplot as plt
import glob
import os
import sys
import argparse


def load_csv_files(folder_path):
    """Load all CSV files from the given folder path."""
    csv_files = glob.glob(os.path.join(folder_path, "*"))
    if not csv_files:
        print(f"No files found in: {folder_path}")
        return None

    dfs = []
    for file in csv_files:
        try:
            df = pd.read_csv(file)
            dfs.append(df)
        except Exception as e:
            print(f"Error reading {file}: {e}")

    if not dfs:
        return None

    return pd.concat(dfs, ignore_index=True)


def process_data(df):
    """Average results grouped by Test ID, Thread Count, and Queuetype."""
    grouped = df.groupby(['Test ID', 'Thread Count', 'Queuetype']).agg({
        'Throughput': 'mean',
        'Fairness': 'mean',
        'Enqueues': 'mean',
        'Dequeues': 'mean'
    }).reset_index()
    return grouped


def plot_results(df):
    """Create plots for each metric,with all Queuetypes layered on the same 4
    graphs."""
    print(df.head())
    metrics = ["Throughput", "Fairness", "Enqueues", "Dequeues"]
    titles = [
        "Throughput vs. Thread Count",
        "Fairness vs. Thread Count",
        "Number of Enqueues vs. Thread Count",
        "Number of Dequeues vs. Thread Count"
    ]

    # Define a set of line styles and marker styles for better distinction
    line_styles = ['-', '--', '-.', ':']
    marker_styles = ['o', 's', 'D', '^', 'v', '<',
                     '>', 'p', '*', 'h', 'H', 'x', '+']

    # Create a single figure with 4 subplots (one for each metric)
    fig, axes = plt.subplots(2, 2, figsize=(15, 10))
    fig.suptitle('Performance Metrics by Queuetype', fontsize=16)

    # Flatten the axes array for easier indexing
    axes = axes.flatten()

    # Plot all Queuetypes on the same 4 graphs
    for i, (metric, title) in enumerate(zip(metrics, titles)):
        for j, qtype in enumerate(df['Queuetype'].unique()):
            queue_data = df[df['Queuetype'] == qtype]
            queue_data = queue_data.sort_values('Thread Count')

            # Cycle through line styles and marker styles
            line_style = line_styles[j % len(line_styles)]
            marker_style = marker_styles[j % len(marker_styles)]

            axes[i].plot(
                queue_data['Thread Count'],
                queue_data[metric],
                marker=marker_style,
                linestyle=line_style,
                label=qtype
            )

        axes[i].set_title(title)
        axes[i].set_xlabel('Thread Count')
        axes[i].set_ylabel(metric)
        axes[i].grid(True)
        axes[i].legend()

    plt.tight_layout()
    plt.show()


def main():
    parser = argparse.ArgumentParser(
            description='Process and plot benchmark data.')
    parser.add_argument('folder',
                        help='Folder containing the benchmark CSV files')
    args = parser.parse_args()

    if not os.path.isdir(args.folder):
        print(f"Error: {args.folder} is not a valid directory")
        sys.exit(1)

    df = load_csv_files(args.folder)
    if df is None:
        print("No valid data was loaded.")
        sys.exit(1)

    processed_df = process_data(df)
    plot_results(processed_df)


if __name__ == "__main__":
    main()
