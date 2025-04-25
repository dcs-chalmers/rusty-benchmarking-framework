import os
import pandas as pd
import numpy as np
import matplotlib.pyplot as plt
import seaborn as sns
import argparse
from glob import glob

def load_benchmark_file(file_path):
    """
    Load a benchmark file and return as DataFrame
    """
    df = pd.read_csv(file_path)
    return df

def calculate_row_means_across_iterations(df):
    """
    Calculate the mean memory allocation for each row position across iterations
    """
    # Get unique row indices in each iteration
    row_indices = df['Row ID'].unique() if 'Row ID' in df.columns else range(len(df[df['Iteration'] == 0]))
    
    # Initialize list to store results
    result_rows = []
    
    # For each row index, calculate mean across iterations
    for row_idx in row_indices:
        if 'Row ID' in df.columns:
            row_data = df[df['Row ID'] == row_idx]
        else:
            # If no Row ID, group by position within each iteration
            iterations = df['Iteration'].unique()
            row_positions = [df[df['Iteration'] == it].iloc[row_idx:row_idx+1] for it in iterations if row_idx < len(df[df['Iteration'] == it])]
            if not row_positions:
                continue
            row_data = pd.concat(row_positions)
        
        # Calculate mean for this row across iterations
        # print(row_data['Memory Allocated'])
        mean_value = row_data['Memory Allocated'].mean()
        # print(f"mean val {mean_value}")
        
        # Get other metadata from first occurrence
        first_row = row_data.iloc[0].copy()
        first_row['Memory Allocated'] = mean_value
        first_row['Row Position'] = row_idx
        
        result_rows.append(first_row)
    
    # Convert to DataFrame
    if result_rows:
        result_df = pd.DataFrame(result_rows)
        return result_df
    else:
        return pd.DataFrame()

def process_queue_folder(folder_path, queue_type=None):
    """
    Process one representative CSV file for a specific queue type
    """
    # Get all CSV files in the folder
    file_paths = glob(os.path.join(folder_path, "mem*"))
    
    if not file_paths:
        print(f"No memory files found in {folder_path}")
        return pd.DataFrame()
    
    # Just use the first file found
    file_path = file_paths[0]
    print(f"Using file {file_path} for queue type {queue_type}")
    
    try:
        df = load_benchmark_file(file_path)
        
        # If queue_type is provided, overwrite the Queuetype column
        if queue_type:
            df['Queuetype'] = queue_type
        
        # Calculate row means across iterations
        means_df = calculate_row_means_across_iterations(df)
        
        # Add file identifier
        means_df['Source'] = os.path.basename(file_path)
        
        return means_df
    except Exception as e:
        print(f"Error processing {file_path}: {e}")
        return pd.DataFrame()

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
        
        # Process one file in the subfolder
        queue_data = process_queue_folder(folder, queue_type)
        
        if not queue_data.empty:
            all_queue_data.append(queue_data)
    
    if all_queue_data:
        return pd.concat(all_queue_data, ignore_index=True)
    else:
        return pd.DataFrame()

def plot_peak_memory_bar_graph(data):
    """
    Create a bar graph showing peak memory usage for each queue type
    """
    # Calculate peak memory for each queue type
    peak_memory = data.groupby('Queuetype')['Memory Allocated'].max().reset_index()
    
    # Sort by peak memory for better visualization
    peak_memory = peak_memory.sort_values('Memory Allocated', ascending=False)
    
    # Create the figure
    plt.figure(figsize=(12, 8))
    
    # Set style
    sns.set_style("whitegrid")
    
    # Create bar plot
    bars = plt.bar(
        x=peak_memory['Queuetype'],
        height=peak_memory['Memory Allocated'],
        width=0.7,
        color=sns.color_palette("viridis", len(peak_memory))
    )
    
    # Add value labels on top of each bar
    for bar in bars:
        height = bar.get_height()
        plt.text(
            bar.get_x() + bar.get_width()/2.,
            height * 1.01,
            f'{int(height):,}',
            ha='center',
            va='bottom',
            fontsize=11,
            rotation=0
        )
    
    # Customize plot
    plt.title('Peak Memory Usage by Queue Type', fontsize=16)
    plt.xlabel('Queue Type', fontsize=14)
    plt.ylabel('Peak Memory Allocated (bytes)', fontsize=14)
    plt.xticks(fontsize=12, rotation=45 if len(peak_memory) > 6 else 0)
    plt.yticks(fontsize=12)
    
    # Add grid for better readability
    plt.grid(axis='y', linestyle='--', alpha=0.7)
    
    # Adjust layout
    plt.tight_layout()
    
    return peak_memory

def main():
    # Set up argument parser
    parser = argparse.ArgumentParser(description='Analyze peak memory usage across different queue types')
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
    
    print("\nPeak memory usage by queue type:")
    for _, row in peak_memory.iterrows():
        print(f"{row['Queuetype']}: {int(row['Memory Allocated']):,} bytes")
    
    print("\nDisplaying interactive plot...")
    plt.show()  # Show plot interactively

if __name__ == "__main__":
    main()
