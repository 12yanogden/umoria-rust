import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "astro/config";

// GitHub project Pages: https://12yanogden.github.io/umoria-rust/
// Exact base string: "/umoria-rust" (no trailing slash; Astro joins paths with leading slashes on routes)
export default defineConfig({
  site: "https://12yanogden.github.io",
  base: "/umoria-rust",
  vite: {
    plugins: [tailwindcss()]
  }
});
