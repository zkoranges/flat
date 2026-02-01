/**
 * Validate input data
 * @param {Object} data - Data to validate
 * @returns {boolean} True if valid
 */
function validate(data) {
  if (!data || typeof data !== 'object') {
    return false;
  }
  return true;
}

/**
 * Validate email format
 */
function validateEmail(email) {
  const regex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  return regex.test(email);
}

module.exports = {
  validate,
  validateEmail
};
