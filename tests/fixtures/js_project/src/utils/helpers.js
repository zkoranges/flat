const { validate } = require('./validators');

/**
 * Process incoming data
 * @param {Object} data - The data to process
 * @returns {Object} Processed data
 */
function processData(data) {
  if (!validate(data)) {
    throw new Error('Invalid data');
  }

  return {
    ...data,
    processed: true,
    timestamp: new Date().toISOString()
  };
}

/**
 * Format response
 */
function formatResponse(data) {
  return {
    success: true,
    data,
    meta: {
      version: '1.0.0'
    }
  };
}

module.exports = {
  processData,
  formatResponse
};
