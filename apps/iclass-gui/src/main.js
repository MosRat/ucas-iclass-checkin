import { createApp } from "vue";
import { takeoverConsole } from "@fltsci/tauri-plugin-tracing";
import App from "./App.vue";
import "./style.css";
import { isTauriRuntime } from "./lib/tauri";
async function bootstrap() {
    if (isTauriRuntime()) {
        try {
            await takeoverConsole();
        }
        catch (error) {
            console.warn("failed to initialize tracing console bridge", error);
        }
    }
    createApp(App).mount("#app");
}
void bootstrap();
