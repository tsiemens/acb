package test

import (
	"fmt"
	"testing"

	"github.com/shopspring/decimal"
	"github.com/stretchr/testify/require"

	"github.com/tsiemens/acb/date"
	"github.com/tsiemens/acb/fx"
	"github.com/tsiemens/acb/log"
)

type MockRemoteRateLoader struct {
	RemoteYearRates map[uint32][]fx.DailyRate
}

func (l *MockRemoteRateLoader) GetRemoteUsdCadRates(year uint32) ([]fx.DailyRate, error) {
	rates, ok := l.RemoteYearRates[year]
	if !ok {
		return nil, fmt.Errorf("No rates set for %v", year)
	}
	return rates, nil
}

func NewTestRateLoaderWithCacheAndRemote(forceDownload bool,
	cache *fx.MemRatesCacheAccessor,
	remoteLoader *MockRemoteRateLoader) *fx.RateLoader {
	errPrinter := &log.StderrErrorPrinter{}
	return &fx.RateLoader{
		YearRates:        make(map[uint32]map[date.Date]fx.DailyRate),
		ForceDownload:    forceDownload,
		Cache:            cache,
		RemoteLoader:     remoteLoader,
		FreshLoadedYears: make(map[uint32]bool),
		ErrPrinter:       errPrinter,
	}
}

func NewTestRateLoaderWithRemote(forceDownload bool,
	remoteLoader *MockRemoteRateLoader) (*fx.RateLoader, *fx.MemRatesCacheAccessor) {
	cache := fx.NewMemRatesCacheAccessor()
	return NewTestRateLoaderWithCacheAndRemote(forceDownload, cache, remoteLoader),
		cache
}

func NewTestRateLoader(forceDownload bool) (
	*fx.RateLoader, *fx.MemRatesCacheAccessor, *MockRemoteRateLoader) {

	cache := fx.NewMemRatesCacheAccessor()
	remoteLoader := &MockRemoteRateLoader{
		RemoteYearRates: make(map[uint32][]fx.DailyRate),
	}
	return NewTestRateLoaderWithCacheAndRemote(forceDownload, cache, remoteLoader),
		cache, remoteLoader
}

func TestFillInUnknownDayRates(t *testing.T) {
	rq := require.New(t)
	crq := NewCustomRequire(t)

	rates := []fx.DailyRate{
		fx.DailyRate{mkDateYD(2022, 0), decimal.NewFromFloat(1.0)},
		fx.DailyRate{mkDateYD(2022, 1), decimal.NewFromFloat(1.1)},
		fx.DailyRate{mkDateYD(2022, 2), decimal.NewFromFloat(1.2)},
	}

	// Simple no fills
	date.TodaysDateForTest = mkDateYD(2022, 2)
	crq.Equal(
		fx.FillInUnknownDayRates(rates, 2022),
		[]fx.DailyRate{
			fx.DailyRate{mkDateYD(2022, 0), decimal.NewFromFloat(1.0)},
			fx.DailyRate{mkDateYD(2022, 1), decimal.NewFromFloat(1.1)},
			fx.DailyRate{mkDateYD(2022, 2), decimal.NewFromFloat(1.2)},
		},
	)

	date.TodaysDateForTest = mkDateYD(2022, 3)
	crq.Equal(
		fx.FillInUnknownDayRates(rates, 2022),
		[]fx.DailyRate{
			fx.DailyRate{mkDateYD(2022, 0), decimal.NewFromFloat(1.0)},
			fx.DailyRate{mkDateYD(2022, 1), decimal.NewFromFloat(1.1)},
			fx.DailyRate{mkDateYD(2022, 2), decimal.NewFromFloat(1.2)},
		},
	)

	// End fill only.
	date.TodaysDateForTest = mkDateYD(2022, 4)
	crq.Equal(
		fx.FillInUnknownDayRates(rates, 2022),
		[]fx.DailyRate{
			fx.DailyRate{mkDateYD(2022, 0), decimal.NewFromFloat(1.0)},
			fx.DailyRate{mkDateYD(2022, 1), decimal.NewFromFloat(1.1)},
			fx.DailyRate{mkDateYD(2022, 2), decimal.NewFromFloat(1.2)},
			fx.DailyRate{mkDateYD(2022, 3), decimal.Zero},
		},
	)

	// Different year
	date.TodaysDateForTest = mkDateYD(2023, 4)
	rq.Equal(
		len(fx.FillInUnknownDayRates(rates, 2022)),
		365,
	)

	// Middle and front fills
	rates = []fx.DailyRate{
		// fx.DailyRate{mkDateYD(2022, 0), 1.0},
		fx.DailyRate{mkDateYD(2022, 1), decimal.NewFromFloat(1.1)},
		// fx.DailyRate{mkDateYD(2022, 2), 1.2},
		fx.DailyRate{mkDateYD(2022, 3), decimal.NewFromFloat(1.3)},
		fx.DailyRate{mkDateYD(2022, 4), decimal.NewFromFloat(1.4)},
		// fx.DailyRate{mkDateYD(2022, 5), 1.2},
		// fx.DailyRate{mkDateYD(2022, 6), 1.2},
		fx.DailyRate{mkDateYD(2022, 7), decimal.NewFromFloat(1.7)},
	}

	date.TodaysDateForTest = mkDateYD(2022, 7)
	crq.Equal(
		fx.FillInUnknownDayRates(rates, 2022),
		[]fx.DailyRate{
			fx.DailyRate{mkDateYD(2022, 0), decimal.Zero},
			fx.DailyRate{mkDateYD(2022, 1), decimal.NewFromFloat(1.1)},
			fx.DailyRate{mkDateYD(2022, 2), decimal.Zero},
			fx.DailyRate{mkDateYD(2022, 3), decimal.NewFromFloat(1.3)},
			fx.DailyRate{mkDateYD(2022, 4), decimal.NewFromFloat(1.4)},
			fx.DailyRate{mkDateYD(2022, 5), decimal.Zero},
			fx.DailyRate{mkDateYD(2022, 6), decimal.Zero},
			fx.DailyRate{mkDateYD(2022, 7), decimal.NewFromFloat(1.7)},
		},
	)

}

func TestGetEffectiveUsdCadRateFreshCache(t *testing.T) {
	rq := require.New(t)
	crq := NewCustomRequire(t)

	date.TodaysDateForTest = mkDateYD(2022, 12)

	rateLoader, ratesCache, remote := NewTestRateLoader(false)
	ratesCache.RatesByYear[2022] = []fx.DailyRate{}
	remote.RemoteYearRates[2022] = []fx.DailyRate{
		fx.DailyRate{mkDateYD(2022, 0), decimal.NewFromFloat(1.0)},
		// fx.DailyRate{mkDateYD(2022, 1), 1.1}, // Expect fill here
		fx.DailyRate{mkDateYD(2022, 2), decimal.NewFromFloat(1.2)},
		// Expect 9 days fill here (unrealistic)
		fx.DailyRate{mkDateYD(2022, 12), decimal.NewFromFloat(2.2)},
	}

	// Test failure to get from remote.
	rate, err := rateLoader.GetEffectiveUsdCadRate(mkDateYD(1970, 1))
	rq.NotNil(err)
	crq.Equal(rate, fx.DailyRate{})

	// Test find exact rate
	rateLoader, ratesCache = NewTestRateLoaderWithRemote(false, remote)
	rate, err = rateLoader.GetEffectiveUsdCadRate(mkDateYD(2022, 0))
	rq.Nil(err)
	crq.Equal(rate, fx.DailyRate{mkDateYD(2022, 0), decimal.NewFromFloat(1.0)})
	// Test fall back to previous day rate
	rateLoader, ratesCache = NewTestRateLoaderWithRemote(false, remote)
	rate, err = rateLoader.GetEffectiveUsdCadRate(mkDateYD(2022, 1))
	rq.Nil(err)
	crq.Equal(rate, fx.DailyRate{mkDateYD(2022, 0), decimal.NewFromFloat(1.0)})
	// Test exact match after a fill day
	rateLoader, ratesCache = NewTestRateLoaderWithRemote(false, remote)
	rate, err = rateLoader.GetEffectiveUsdCadRate(mkDateYD(2022, 2))
	rq.Nil(err)
	crq.Equal(rate, fx.DailyRate{mkDateYD(2022, 2), decimal.NewFromFloat(1.2)})
	// Test fall back with the day ahead also not being present
	rateLoader, ratesCache = NewTestRateLoaderWithRemote(false, remote)
	rate, err = rateLoader.GetEffectiveUsdCadRate(mkDateYD(2022, 7))
	rq.Nil(err)
	crq.Equal(rate, fx.DailyRate{mkDateYD(2022, 2), decimal.NewFromFloat(1.2)})
	// Test fall back 7 days (the max allowed)
	rateLoader, ratesCache = NewTestRateLoaderWithRemote(false, remote)
	rate, err = rateLoader.GetEffectiveUsdCadRate(mkDateYD(2022, 9))
	rq.Nil(err)
	crq.Equal(rate, fx.DailyRate{mkDateYD(2022, 2), decimal.NewFromFloat(1.2)})
	// Test fall back 8 days (more than allowed)
	rateLoader, ratesCache = NewTestRateLoaderWithRemote(false, remote)
	rate, err = rateLoader.GetEffectiveUsdCadRate(mkDateYD(2022, 10))
	rq.NotNil(err)
	crq.Equal(rate, (fx.DailyRate{}))

	// Test lookup for today's determined (markets opened) rate
	rateLoader, ratesCache = NewTestRateLoaderWithRemote(false, remote)
	rate, err = rateLoader.GetEffectiveUsdCadRate(mkDateYD(2022, 12))
	rq.Nil(err)
	crq.Equal(rate, fx.DailyRate{mkDateYD(2022, 12), decimal.NewFromFloat(2.2)})

	// Test lookup for today's (undetermined) rate
	date.TodaysDateForTest = mkDateYD(2022, 13)

	rateLoader, ratesCache = NewTestRateLoaderWithRemote(false, remote)
	rate, err = rateLoader.GetEffectiveUsdCadRate(mkDateYD(2022, 13))
	rq.NotNil(err)
	crq.Equal(rate, (fx.DailyRate{}))

	// Test lookup for yesterday's determined (markets closed) rate
	date.TodaysDateForTest = mkDateYD(2022, 14)

	rateLoader, ratesCache = NewTestRateLoaderWithRemote(false, remote)
	rate, err = rateLoader.GetEffectiveUsdCadRate(mkDateYD(2022, 13))
	rq.Nil(err)
	crq.Equal(rate, fx.DailyRate{mkDateYD(2022, 12), decimal.NewFromFloat(2.2)})
}

func TestGetEffectiveUsdCadRateWithCache(t *testing.T) {
	rq := require.New(t)
	crq := NewCustomRequire(t)

	// Sanity check
	rq.True(
		fx.DailyRate{mkDateYD(2022, 1), decimal.NewFromFloat(0.0)}.Equal(
			fx.DailyRate{mkDateYD(2022, 1), decimal.Zero}))

	rateLoader, ratesCache, _ := NewTestRateLoader(false)
	ratesCache.RatesByYear[2022] = []fx.DailyRate{
		fx.DailyRate{mkDateYD(2022, 1), decimal.NewFromFloat(1.1)},
		fx.DailyRate{mkDateYD(2022, 2), decimal.NewFromFloat(0.0)}, // Filled (markets closed)
		fx.DailyRate{mkDateYD(2022, 3), decimal.NewFromFloat(0.0)}, // Filled (markets closed)
	}

	// Test lookup of well-known cached value for tomorrow, today, yesterday
	for i := 0; i <= 2; i++ {
		date.TodaysDateForTest = mkDateYD(2022, i)
		rate, err := rateLoader.GetEffectiveUsdCadRate(mkDateYD(2022, 1))
		rq.Nil(err)
		crq.Equal(rate, fx.DailyRate{mkDateYD(2022, 1), decimal.NewFromFloat(1.1)})
	}
	// Test lookup of defined markets closed cached value for tomorrow, today, yesterday,
	// where later values are present.
	for i := 1; i <= 3; i++ {
		date.TodaysDateForTest = mkDateYD(2022, i)
		rate, err := rateLoader.GetEffectiveUsdCadRate(mkDateYD(2022, 2))
		rq.Nil(err)
		crq.Equal(rate, fx.DailyRate{mkDateYD(2022, 1), decimal.NewFromFloat(1.1)})
	}
	// Test lookup of defined markets closed cached value for tomorrow, today, yesterday,
	// where this is the last cached value.
	for i := 2; i <= 4; i++ {
		date.TodaysDateForTest = mkDateYD(2022, i)
		rate, err := rateLoader.GetEffectiveUsdCadRate(mkDateYD(2022, 3))
		rq.Nil(err)
		crq.Equal(rate, fx.DailyRate{mkDateYD(2022, 1), decimal.NewFromFloat(1.1)})
	}
}

func TestGetEffectiveUsdCadRateCacheInvalidation(t *testing.T) {
	rq := require.New(t)
	crq := NewCustomRequire(t)

	// Test cache invalidates when querying today with no cached value, and there is
	// no remote value
	rateLoader, ratesCache, remote := NewTestRateLoader(false)
	ratesCache.RatesByYear[2022] = []fx.DailyRate{
		fx.DailyRate{mkDateYD(2022, 1), decimal.NewFromFloat(1.1)},
	}
	remote.RemoteYearRates[2022] = []fx.DailyRate{
		fx.DailyRate{mkDateYD(2022, 1), decimal.NewFromFloat(1.4)}, // Value change.
	}
	date.TodaysDateForTest = mkDateYD(2022, 2)
	_, err := rateLoader.GetEffectiveUsdCadRate(mkDateYD(2022, 2))
	crq.Equal(ratesCache.RatesByYear[2022],
		[]fx.DailyRate{
			fx.DailyRate{mkDateYD(2022, 0), decimal.NewFromFloat(0.0)}, // fill
			fx.DailyRate{mkDateYD(2022, 1), decimal.NewFromFloat(1.4)},
		})
	rq.NotNil(err) // Can't use today unless it's been published or specified.

	// Test cache invalidates when querying today with no cached value, and there is
	// a remote value
	rateLoader, ratesCache, remote = NewTestRateLoader(false)
	ratesCache.RatesByYear[2022] = []fx.DailyRate{
		fx.DailyRate{mkDateYD(2022, 0), decimal.NewFromFloat(1.0)},
	}
	remote.RemoteYearRates[2022] = []fx.DailyRate{
		fx.DailyRate{mkDateYD(2022, 0), decimal.NewFromFloat(1.0)},
		fx.DailyRate{mkDateYD(2022, 1), decimal.NewFromFloat(1.1)},
	}
	date.TodaysDateForTest = mkDateYD(2022, 1)
	rate, err := rateLoader.GetEffectiveUsdCadRate(mkDateYD(2022, 1))
	rq.Nil(err)
	crq.Equal(ratesCache.RatesByYear[2022], remote.RemoteYearRates[2022])
	crq.Equal(rate, fx.DailyRate{mkDateYD(2022, 1), decimal.NewFromFloat(1.1)})

	// Test cache invalidates when querying a previous day with no cached value.
	rateLoader, ratesCache, remote = NewTestRateLoader(false)
	ratesCache.RatesByYear[2022] = []fx.DailyRate{
		fx.DailyRate{mkDateYD(2022, 0), decimal.NewFromFloat(1.0)},
	}
	remote.RemoteYearRates[2022] = []fx.DailyRate{
		fx.DailyRate{mkDateYD(2022, 0), decimal.NewFromFloat(1.0)},
		fx.DailyRate{mkDateYD(2022, 1), decimal.NewFromFloat(1.1)},
	}
	date.TodaysDateForTest = mkDateYD(2022, 4)
	rate, err = rateLoader.GetEffectiveUsdCadRate(mkDateYD(2022, 1))
	rq.Nil(err)
	crq.Equal(ratesCache.RatesByYear[2022],
		[]fx.DailyRate{
			fx.DailyRate{mkDateYD(2022, 0), decimal.NewFromFloat(1.0)},
			fx.DailyRate{mkDateYD(2022, 1), decimal.NewFromFloat(1.1)},
			fx.DailyRate{mkDateYD(2022, 2), decimal.NewFromFloat(0.0)}, // fill to yesterday
			fx.DailyRate{mkDateYD(2022, 3), decimal.NewFromFloat(0.0)}, // fill to yesterday
		})
	crq.Equal(rate, fx.DailyRate{mkDateYD(2022, 1), decimal.NewFromFloat(1.1)})

	// Test cache does not invalidate when querying today with no cached value,
	// after we already invalidated and refreshed the cache with this Loader instance.
	remote.RemoteYearRates[2022] = []fx.DailyRate{
		fx.DailyRate{mkDateYD(2022, 0), decimal.NewFromFloat(99.0)},
		fx.DailyRate{mkDateYD(2022, 1), decimal.NewFromFloat(99.1)},
	}
	_, err = rateLoader.GetEffectiveUsdCadRate(mkDateYD(2022, 4))
	// Cache should be unchanged
	crq.Equal(ratesCache.RatesByYear[2022],
		[]fx.DailyRate{
			fx.DailyRate{mkDateYD(2022, 0), decimal.NewFromFloat(1.0)},
			fx.DailyRate{mkDateYD(2022, 1), decimal.NewFromFloat(1.1)},
			fx.DailyRate{mkDateYD(2022, 2), decimal.NewFromFloat(0.0)},
			fx.DailyRate{mkDateYD(2022, 3), decimal.NewFromFloat(0.0)},
		})
	rq.NotNil(err) // Can't use today unless it's been published or specified.

	// Test force download
	rateLoader, ratesCache, remote = NewTestRateLoader(false)
	rateLoader.ForceDownload = true
	ratesCache.RatesByYear[2022] = []fx.DailyRate{
		fx.DailyRate{mkDateYD(2022, 0), decimal.NewFromFloat(1.0)},
	}
	remote.RemoteYearRates[2022] = []fx.DailyRate{
		fx.DailyRate{mkDateYD(2022, 0), decimal.NewFromFloat(99.0)},
	}
	date.TodaysDateForTest = mkDateYD(2022, 1)
	rate, err = rateLoader.GetEffectiveUsdCadRate(mkDateYD(2022, 0))
	rq.Nil(err)
	crq.Equal(ratesCache.RatesByYear[2022], remote.RemoteYearRates[2022])
	crq.Equal(rate, fx.DailyRate{mkDateYD(2022, 0), decimal.NewFromFloat(99.0)})
}
