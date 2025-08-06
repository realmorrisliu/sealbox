import { useState, useEffect } from "react";
import { useTranslation } from "react-i18next";

interface CountdownTimerProps {
  expiresAt: number; // Unix timestamp
  className?: string;
}

export function CountdownTimer({ expiresAt, className }: CountdownTimerProps) {
  const { t } = useTranslation();
  const [timeLeft, setTimeLeft] = useState<string>("");
  const [isExpired, setIsExpired] = useState(false);
  const [isWarning, setIsWarning] = useState(false);

  useEffect(() => {
    const updateCountdown = () => {
      const now = Date.now() / 1000;
      const diff = expiresAt - now;

      if (diff <= 0) {
        setTimeLeft(t('common.expired'));
        setIsExpired(true);
        setIsWarning(false);
        return;
      }

      // Check if expiring soon (within 7 days)
      const warningThreshold = 7 * 24 * 60 * 60; // 7 days in seconds
      setIsWarning(diff <= warningThreshold);
      setIsExpired(false);

      // Format time remaining
      const days = Math.floor(diff / (24 * 60 * 60));
      const hours = Math.floor((diff % (24 * 60 * 60)) / (60 * 60));
      const minutes = Math.floor((diff % (60 * 60)) / 60);

      if (days > 0) {
        setTimeLeft(t('common.timeLeft.days', { count: days }));
      } else if (hours > 0) {
        setTimeLeft(t('common.timeLeft.hours', { count: hours }));
      } else if (minutes > 0) {
        setTimeLeft(t('common.timeLeft.minutes', { count: minutes }));
      } else {
        setTimeLeft(t('common.timeLeft.lessThanMinute'));
      }
    };

    // Update immediately
    updateCountdown();

    // Update every minute
    const interval = setInterval(updateCountdown, 60000);

    return () => clearInterval(interval);
  }, [expiresAt, t]);

  const getTextColor = () => {
    if (isExpired) return "text-red-600";
    if (isWarning) return "text-yellow-600";
    return "text-green-600";
  };

  return (
    <span className={`${getTextColor()} ${className}`}>
      {timeLeft}
    </span>
  );
}