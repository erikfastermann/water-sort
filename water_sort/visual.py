import json
import argparse
import sys
import matplotlib.pyplot as plt
from matplotlib.widgets import Button

# Color map
COLOR_MAP = {
    "R": "#FF4B4B",   # Vibrant Red
    "G": "#2ECC71",   # Vibrant Green
    "B": "#3498DB",   # Vibrant Blue
    "Y": "#F1C40F",   # Sunflower Yellow
    "S": "#000000",   # Black
    "W": "#BDC3C7",   # Gray
    "NA": "#FFFFFF"   # Empty slot
}

def parse_args():
    parser = argparse.ArgumentParser(description="Visualize MiniZinc Water Sort Puzzle Trajectory.")
    parser.add_argument("-i", "--input", required=True, help="Path to the input JSON file")
    parser.add_argument("-s", "--solution", required=True, help="Path to the solution JSON file")
    return parser.parse_args()

def load_json_files(input_path, solution_path):
    try:
        with open(input_path, 'r') as f:
            input_data = json.load(f)
        with open(solution_path, 'r') as f:
            sol_raw = json.load(f)
    except FileNotFoundError as e:
        print(f"Error: {e}")
        sys.exit(1)
    except json.JSONDecodeError as e:
        print(f"Error parsing JSON: {e}")
        sys.exit(1)

    # Unpack the deep MiniZinc IDE output structure safely
    try:
        sol_data = sol_raw["output"]["json"]
    except KeyError:
        print("Error: Solution JSON doesn't match the expected structural format 'output.json'")
        sys.exit(1)

    return input_data, sol_data

class WaterSortVisualizer:
    def __init__(self, input_data, sol_data):
        self.input_data = input_data
        self.sol_data = sol_data

        self.bottle_count = input_data["BottleCount"]
        self.bottle_size = input_data["BottleSize"]
        self.max_steps = sol_data["UsedTime"]
        self.current_step = 0

        # Extract configurations out of solver response
        self.config_history = sol_data["Config"]
        self.fill_history = sol_data["FillHeight"]

        # Build interactive figure frame
        self.fig, self.ax = plt.subplots(figsize=(9, 5.5))
        self.fig.canvas.manager.set_window_title('Water Sort Solver Playback')
        plt.subplots_adjust(bottom=0.2)

        self.draw_step()

        # Attach interaction UI components
        ax_prev = plt.axes([0.33, 0.04, 0.14, 0.065])
        ax_next = plt.axes([0.53, 0.04, 0.14, 0.065])

        self.btn_prev = Button(ax_prev, '◀ Prev')
        self.btn_next = Button(ax_next, 'Next ▶')

        self.btn_prev.on_clicked(self.prev_click)
        self.btn_next.on_clicked(self.next_click)

    def clean_color_token(self, cell_data):
        """Unwraps MiniZinc enum structures like {'e': 'R'} to string keys safely."""
        if isinstance(cell_data, dict) and "e" in cell_data:
            return cell_data["e"]
        return str(cell_data)

    def draw_step(self):
        self.ax.clear()

        # Isolate step array layer
        current_config = self.config_history[self.current_step]
        current_fill = self.fill_history[self.current_step]

        for b_idx in range(self.bottle_count):
            bottle_x = b_idx * 2
            fill_height = current_fill[b_idx]

            # Draw individual block tiers
            for h_idx in range(self.bottle_size):
                # Only map actual fluids up to fill height bounds
                if h_idx < fill_height:
                    raw_token = current_config[b_idx][h_idx]
                    color_key = self.clean_color_token(raw_token)
                    face_color = COLOR_MAP.get(color_key, "#BDC3C7") # gray fallback if token missing
                else:
                    face_color = COLOR_MAP["NA"]

                rect = plt.Rectangle((bottle_x + 0.2, h_idx), 1.6, 1.0,
                                     facecolor=face_color, edgecolor="#E2E8F0", linewidth=1)
                self.ax.add_patch(rect)

            # Render outer glass frame bounds
            outer_bottle = plt.Rectangle((bottle_x + 0.15, 0), 1.7, self.bottle_size,
                                         facecolor="none", edgecolor="#1E293B", linewidth=2.5)
            self.ax.add_patch(outer_bottle)
            self.ax.text(bottle_x + 1.0, -0.4, f"B{b_idx+1}", ha='center', fontweight='bold', color="#475569")

        # Context Header Strings
        if self.current_step == 0:
            title_text = f"Step 0 / {self.max_steps} : Initial Grid State"
        else:
            fb = self.sol_data["FromBottle"][self.current_step - 1]
            tb = self.sol_data["ToBottle"][self.current_step - 1]
            title_text = f"Step {self.current_step} / {self.max_steps} : Poured Bottle {fb} ➔ Bottle {tb}"

        self.ax.set_title(title_text, fontsize=12, fontweight='bold', pad=15, color="#0F172A")
        self.ax.set_xlim(-1, self.bottle_count * 2)
        self.ax.set_ylim(-0.8, self.bottle_size + 0.5)
        self.ax.axis('off')
        plt.draw()

    def next_click(self, event):
        if self.current_step < self.max_steps:
            self.current_step += 1
            self.draw_step()

    def prev_click(self, event):
        if self.current_step > 0:
            self.current_step -= 1
            self.draw_step()

if __name__ == "__main__":
    args = parse_args()
    input_data, sol_data = load_json_files(args.input, args.solution)

    visualizer = WaterSortVisualizer(input_data, sol_data)
    plt.show()
