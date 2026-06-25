import type { NextConfig } from "next";

const nextConfig: NextConfig = {
  // The repo root also has a lockfile (the Tauri app), so pin this app's root
  // to its own directory for deterministic local builds and deploys.
  turbopack: {
    root: __dirname,
  },
};

export default nextConfig;
