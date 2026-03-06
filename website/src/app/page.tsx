import Nav from "@/components/Nav";
import Hero from "@/components/Hero";
import Problem from "@/components/Problem";
import LiveFeeds from "@/components/LiveFeeds";
import HowItWorks from "@/components/HowItWorks";
import ZeroMigration from "@/components/ZeroMigration";
import CostComparison from "@/components/CostComparison";
import Latency from "@/components/Latency";
import BeyondBasic from "@/components/BeyondBasic";

import Footer from "@/components/Footer";

export default function Home() {
  return (
    <main className="min-h-screen bg-bg">
      <Nav />
      <Hero />
      <Problem />
      <LiveFeeds />
      <HowItWorks />
      <ZeroMigration />
      <CostComparison />
      <Latency />
      <BeyondBasic />

      <Footer />
    </main>
  );
}
