export default function Latency() {
  return (
    <section className="py-28 px-6 relative overflow-hidden">
      <div className="absolute top-0 left-1/2 -translate-x-1/2 w-[500px] h-[500px] bg-neon-cyan/5 rounded-full blur-[200px]" />

      <div className="max-w-5xl mx-auto relative z-10">
        <div className="text-center mb-16">
          <h2 className="text-4xl sm:text-6xl font-display tracking-wide">
            <span className="block mb-2"><span className="text-neon-cyan text-glow-cyan">15 seconds</span><span className="text-text-primary"> to prove.</span></span>
            <span className="block"><span className="text-neon-green text-glow-green">12 seconds</span><span className="text-text-primary"> to land.</span></span>
          </h2>
        </div>

        {/* Timeline bar */}
        <div className="max-w-2xl mx-auto mb-16">
          <div className="h-4 bg-white/5 rounded-full overflow-hidden mb-3">
            <div className="h-full flex">
              <div className="w-[55%] rounded-l-full" style={{ background: "linear-gradient(90deg, #00e5ff, #4d7cff)" }} />
              <div className="w-[45%] rounded-r-full" style={{ background: "linear-gradient(90deg, #39ff14, #00e5ff)" }} />
            </div>
          </div>
          <div className="flex justify-between text-xs sm:text-sm font-mono">
            <span className="text-neon-cyan">0s</span>
            <span className="text-neon-blue">15s proved</span>
            <span className="text-neon-green">~27s on-chain</span>
          </div>
        </div>

        <div className="grid md:grid-cols-2 gap-6 max-w-4xl mx-auto mb-12">
          <div className="glass-strong rounded-2xl p-8 glow-cyan">
            <div className="text-xs font-mono text-neon-cyan tracking-widest mb-3">ETHEREUM L1</div>
            <div className="text-5xl font-mono font-black text-text-primary mb-2">~27s</div>
            <p className="text-text-secondary text-sm font-body">15s proving + 12s block inclusion</p>
          </div>
          <div className="glass-strong rounded-2xl p-8 glow-green">
            <div className="text-xs font-mono text-neon-green tracking-widest mb-3">L2 (BASE / ARBITRUM)</div>
            <div className="text-5xl font-mono font-black text-text-primary mb-2">~16s</div>
            <p className="text-text-secondary text-sm font-body">15s proving + ~1s block inclusion</p>
          </div>
        </div>

        <div className="glass-strong rounded-2xl p-8 max-w-3xl mx-auto text-center">
          <p className="text-text-secondary font-body text-lg">
            Chainlink&apos;s push feeds for long-tail tokens have{" "}
            <span className="text-text-primary font-bold">1-hour to 24-hour heartbeats</span>.
            For the 499,650 pairs they don&apos;t cover, 15 seconds is fast.
          </p>
        </div>
      </div>
    </section>
  );
}
