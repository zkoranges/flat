const { processData, formatResponse } = require('../../src/utils/helpers');

describe('processData', () => {
  test('should process valid data', () => {
    const input = { name: 'test' };
    const result = processData(input);

    expect(result.processed).toBe(true);
    expect(result.name).toBe('test');
    expect(result.timestamp).toBeDefined();
  });

  test('should throw on invalid data', () => {
    expect(() => processData(null)).toThrow('Invalid data');
  });
});

describe('formatResponse', () => {
  test('should format response correctly', () => {
    const data = { id: 1 };
    const result = formatResponse(data);

    expect(result.success).toBe(true);
    expect(result.data).toEqual(data);
    expect(result.meta.version).toBe('1.0.0');
  });
});
