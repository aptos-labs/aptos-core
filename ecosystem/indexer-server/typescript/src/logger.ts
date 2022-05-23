import { createLogger, format, transports } from 'winston';

const logger = createLogger({
  defaultMeta: { service: 'user-service' },
  format: format.combine(
    format.timestamp(),
    format.json(),
  ),
  level: 'info',
  transports: [
    new transports.Console(),
    new transports.File({ filename: 'error.log', level: 'error', timestamp: true }),
    new transports.File({ filename: 'combined.log', timestamp: true }),
  ],
});

if (process.env.NODE_ENV !== 'production') {
  logger.add(new transports.Console({ format: format.simple() }));
}

export default logger;
