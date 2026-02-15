CREATE FUNCTION calculate_totals(user_id INT)
RETURNS TABLE (total DECIMAL, count INT) AS $$
BEGIN
    RETURN QUERY
    SELECT SUM(amount), COUNT(*)
    FROM orders o
    INNER JOIN order_items oi ON o.id = oi.order_id
    WHERE o.user_id = calculate_totals.user_id
    GROUP BY o.user_id;
END;
$$ LANGUAGE plpgsql;
