import Image from "next/image";
import { ShaderBackground } from "./shader-background";

const VERSION = "0.1.0";
const DMG_PATH = `/downloads/Whop_${VERSION}_universal.dmg`;
const GITHUB_URL = "https://github.com/siyabendoezdemir/whop-desktop";

function GitHubIcon({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 16 16"
      aria-hidden="true"
      fill="currentColor"
      className={className}
    >
      <path d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82.64-.18 1.32-.27 2-.27.68 0 1.36.09 2 .27 1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8Z" />
    </svg>
  );
}

export default function Home() {
  return (
    <main className="relative flex min-h-screen flex-col items-center justify-center px-6">
      <ShaderBackground />

      <div className="flex w-full max-w-md flex-col items-center text-center">
        <Image
          src="/whop-mark.png"
          alt="Whop"
          width={72}
          height={72}
          priority
          className="h-[72px] w-[72px] rounded-[22%]"
        />

        <h1 className="mt-8 text-4xl font-medium tracking-tight text-foreground sm:text-5xl">
          Whop on your Mac
        </h1>

        <p className="mt-4 text-base leading-relaxed text-dust">
          A native desktop app for whop.com. It opens in its own window and
          stays signed in.
        </p>

        <div className="mt-9 flex flex-wrap items-center justify-center gap-3">
          <a
            href={DMG_PATH}
            download
            className="inline-flex items-center justify-center rounded-full bg-vermilion px-6 py-3 text-sm font-medium text-white transition-colors hover:bg-[#e23d10] focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-vermilion"
          >
            Download for macOS
          </a>
          <a
            href={GITHUB_URL}
            target="_blank"
            rel="noreferrer"
            className="inline-flex items-center justify-center gap-2 rounded-full border border-white/15 px-6 py-3 text-sm font-medium text-foreground transition-colors hover:border-white/30 hover:bg-white/5 focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-white/40"
          >
            <GitHubIcon className="h-4 w-4" />
            GitHub
          </a>
        </div>

        <p className="mt-4 text-xs text-dust/70">
          v{VERSION} · Universal (Apple Silicon &amp; Intel) · macOS 11+
        </p>

        <p className="mt-10 text-xs leading-relaxed text-dust/70">
          Open the download and drag <span className="text-foreground">Whop</span>{" "}
          into Applications. That&apos;s it — it&apos;s signed and notarized by
          Apple, so it just opens.
        </p>
      </div>

      <footer className="absolute inset-x-0 bottom-7 px-6 text-center">
        <p className="mx-auto max-w-md text-xs leading-relaxed text-dust/45">
          Unofficial and for personal use. Not affiliated with Whop.
        </p>
      </footer>
    </main>
  );
}
