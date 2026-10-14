import json
import numpy as np
from concorde import UncertainValue


def test_exact_arrays_have_no_sources_and_keep_shape():
    for values in [np.array([]), np.array([1., 2., 4.])]:
        x = UncertainValue.exact(values)
        assert x.variable_id is None
        assert json.loads(x.to_json())["sources"] == []
        np.testing.assert_array_equal(x.uncertainty, np.zeros_like(values))
        assert x.sum().uncertainty == 0.
        np.testing.assert_array_equal(UncertainValue.from_json(x.to_json()).uncertainty, np.zeros_like(values))


def test_exact_values_broadcast_shared_uncertainty_without_inventing_sources():
    x = UncertainValue.exact(np.array([1., 2.]))
    shared = UncertainValue(2., .2)
    y = x+shared
    np.testing.assert_allclose(y.uncertainty, [.2, .2])
    assert (y[0]-y[1]).uncertainty == 0.
    assert (y.sum()-2*shared).uncertainty == 0.
    assert UncertainValue.exact(0.).sqrt().uncertainty == 0.
    np.testing.assert_array_equal((x-x).uncertainty, [0., 0.])
