import matplotlib.pyplot as plt
import pandas as pd
import sys
import os
import glob

def load_all_csv(folder_path):  
    folder_path = os.path.abspath(folder_path) 
    all_files = glob.glob(os.path.join(folder_path, "**", "*"), recursive=True) 

    if not all_files:
        print(f"No files found in the folder: {folder_path}")
        sys.exit(1)

    df_list = []
    for file in all_files:
        try:
            df = pd.read_csv(file)  
            df_list.append(df)
            print(f"Loaded: {file}")  
        except pd.errors.ParserError:
            print(f"Skipping (not a CSV file): {file}")
        except Exception as e:
            print(f"Error loading {file}: {e}")

    if not df_list:
        print("No valid CSV files found.")
        sys.exit(1)

    merged_df = pd.concat(df_list, ignore_index=True)
    return merged_df

def plot_graph(df):
    if "Throughput" in df.columns and "Thread Count" in df.columns:
        df = df.sort_values(by="Thread Count")  
        df["Thread Count"] = df["Thread Count"].astype(int)  

        plt.figure(figsize=(10, 5))
        plt.plot(df["Thread Count"], df["Throughput"], marker='o', linestyle='-', label="Throughput")
        plt.title('Throughput vs. Thread Count')
        plt.xlabel("Thread Count")
        plt.ylabel("Throughput")
        plt.xticks(df["Thread Count"]) 
        plt.legend()
        plt.grid()
        plt.show()
    else:
        print("Required columns not found in CSV files.")

def calc_average(df, output_file):
    avg = df.groupby("Thread Count", as_index=False)["Throughput"].mean()  

    avg.to_csv(output_file, index=False)
    print(f"Averaged throughput data saved to {output_file}")

if __name__ == "__main__":
    folder_path = input("Enter the folder containing CSV files: ")
    
    if not os.path.isdir(folder_path):
        print("Invalid folder path.")
        sys.exit(1)

    df = load_all_csv(folder_path)

    output_file = input("Enter output file name: ")
    calc_average(df, output_file)

    df_out = pd.read_csv(output_file)
    print("CSV Files Merged Successfully! Generating Graphs...")
    plot_graph(df_out)
