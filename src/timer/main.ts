import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";

const appWindow = getCurrentWindow();
const count = document.getElementById("count") as HTMLDivElement;

async function main() {
  let remaining = await invoke<number>("timer_duration");
  count.textContent = String(remaining);

  const tick = window.setInterval(() => {
    remaining -= 1;
    if (remaining <= 0) {
      window.clearInterval(tick);
      // Rust destroys this window before capturing, so no cleanup here.
      void invoke("timed_capture_fire");
    } else {
      count.textContent = String(remaining);
    }
  }, 1000);

  const cancel = () => {
    window.clearInterval(tick);
    void appWindow.close();
  };
  document.addEventListener("click", cancel);
  document.addEventListener("keydown", (event) => {
    if (event.key === "Escape") cancel();
  });
}

void main();
