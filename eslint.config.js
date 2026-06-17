import js from "@eslint/js";
import tseslint from "typescript-eslint";

export default tseslint.config(
    js.configs.recommended,
    ...tseslint.configs.recommended,
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
    },
    {
        ignores: ["dist/**", "node_modules/**", ".firecrawl/**", "apps/frontend/.next/**", "apps/frontend/next-env.d.ts"]
    }
);
