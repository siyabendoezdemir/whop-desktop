import Image from "next/image";
import { ShaderBackground } from "./shader-background";

const VERSION = "0.1.0";
const DMG_PATH = `/downloads/Whop_${VERSION}_universal.dmg`;

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

        <a
          href={DMG_PATH}
          download
          className="mt-9 inline-flex items-center justify-center rounded-full bg-vermilion px-6 py-3 text-sm font-medium text-white transition-colors hover:bg-[#e23d10] focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-vermilion"
        >
          Download for macOS
        </a>

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
