import type { Config } from "tailwindcss";

export default {
  content: ["./index.html", "./src/**/*.{ts,vue}"],
  theme: {
    extend: {
      colors: {
        accent: {
          50: "#eef6ff",
          100: "#d9ebff",
          200: "#b8d9ff",
          300: "#86bcff",
          400: "#569cff",
          500: "#2f7df6",
          600: "#1d62d8",
          700: "#184eae",
          800: "#1a448c",
          900: "#193a74"
        },
        ink: {
          50: "#f4f7fb",
          100: "#e7edf5",
          200: "#ced9e8",
          300: "#a8bad2",
          400: "#7d95b6",
          500: "#5f7697",
          600: "#4a5f7d",
          700: "#3d4d66",
          800: "#354156",
          900: "#313949"
        }
      },
      boxShadow: {
        fluent: "0 24px 60px rgba(17, 31, 58, 0.18)",
        pane: "0 16px 40px rgba(27, 46, 89, 0.12)"
      },
      borderRadius: {
        "4xl": "2rem"
      },
      fontFamily: {
        sans: ["'Segoe UI Variable Text'", "'Segoe UI'", "'Noto Sans SC'", "sans-serif"]
      }
    }
  },
  plugins: []
} satisfies Config;

