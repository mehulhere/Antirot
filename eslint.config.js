import js from "@eslint/js";
import tseslint from "typescript-eslint";

export default tseslint.config(
    {
        ignores: [
            "dist/**",
            "node_modules/**",
            ".firecrawl/**",
            "apps/frontend/.next/**",
            "apps/frontend/next-env.d.ts",
            "apps/frontend/public/vad/**"
        ]
    },
    js.configs.recommended,
    ...tseslint.configs.recommended,
    {
        files: ["**/*.js", "**/*.mjs"],
        languageOptions: {
            globals: {
                AbortController: "readonly",
                Blob: "readonly",
                Buffer: "readonly",
                FormData: "readonly",
                Headers: "readonly",
                URL: "readonly",
                clearTimeout: "readonly",
                console: "readonly",
                fetch: "readonly",
                process: "readonly",
                setTimeout: "readonly"
            }
        }
    },
    {
        files: ["**/*.ts", "**/*.tsx"],
        languageOptions: {
            parserOptions: {
                projectService: {
                    allowDefaultProject: ["scripts/*.ts"]
                },
                tsconfigRootDir: import.meta.dirname
            }
        },
        rules: {
            "@typescript-eslint/consistent-type-imports": "error",
            "@typescript-eslint/no-explicit-any": "error",
            "no-console": "off"
        }
    }
);
