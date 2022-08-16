import Script from 'next/script';

const GA_MEASUREMENT_ID = 'G-YRXZ45KCLF';

interface AnalyticsLayoutProps {
  children: React.ReactNode;
}

export default function AnalyticsLayout({
  children,
}: AnalyticsLayoutProps) {
  return (
    <div>
      {/* https://nextjs.org/docs/messages/next-script-for-ga */}
      {/* Global site tag (gtag.js) - Google Analytics */}
      <Script
        src={`https://www.googletagmanager.com/gtag/js?id=${GA_MEASUREMENT_ID}`}
        strategy="afterInteractive"
      />
      <Script id="google-analytics" strategy="afterInteractive">
        {`
          window.dataLayer = window.dataLayer || [];
          function gtag(){window.dataLayer.push(arguments);}
          gtag('js', new Date());

          gtag('config', '${GA_MEASUREMENT_ID}');
        `}
      </Script>
      {children}
    </div>
  );
}
